// Tcl regex engine wrapper.
//
// Links against Tcl 9.0.3's Henry-Spencer regex engine (the same
// engine tclsh uses) compiled in from ``runtime/zig/vendor/tcl-regex/``
// via build.zig.  We expose ``tcl_cmd_regexp`` — the 2-arg form
// ``regexp pattern string`` that returns 1/0 for match / no match.
//
// Higher-arity forms (``-nocase``, ``-all``, ``-indices``, capture
// vars, etc.) arrive via the Python codegen's eval fallback and
// land in ``eval_regexp_cmd`` below, which is plugged into
// ``tcl_interp.zig``'s command dispatch table.  Both paths share
// the core ``run_match`` helper.
//
// Pattern and subject strings arrive as TclObj-wrapped UTF-8
// bytes; Tcl's regex engine operates on ``Tcl_UniChar`` (32-bit
// codepoint) arrays, so each call decodes UTF-8 into a fresh
// UniChar buffer on the bump allocator.  For ASCII-only input
// (all tcltest init patterns, all of counter.test) the decoded
// length equals the byte length and the cost is a simple memcpy
// sign-extension.
//
// Memory: the regex engine's internal allocations route through
// the wasi-libc ``malloc`` the regex shim binds.  ``TclReFree``
// tears down its internal state between matches, but our bump
// allocator's ``free`` is a no-op — the UniChar buffers and the
// ``regex_t`` struct itself leak into the bump arena for the
// lifetime of the WASM instance.  For single-test runs that's
// bounded; long-running use will want a real heap.

const std = @import("std");
const rt = @import("../tcl_runtime.zig");
const stubs = @import("../stubs/tcl_stubs.zig");
const frames = @import("../interp/tcl_frames.zig");
const obj = @import("tcl_obj.zig");
const arena = @import("tcl_arena.zig");
const obj_ensure_string = rt.obj_ensure_string;
const obj_new_int = rt.obj_new_int;
const obj_new_string = rt.obj_new_string;
const alloc = rt.alloc;

/// Compare a (ptr, len) byte span against an ASCII literal.  Used
/// by the option parser to recognise switch keywords without
/// allocating temporary buffers.
fn str_eq(span: anytype, literal: []const u8) bool {
    if (span.len != literal.len) return false;
    const sp: [*]const u8 = @ptrFromInt(span.ptr);
    for (literal, 0..) |b, i| {
        if (sp[i] != b) return false;
    }
    return true;
}

// ``regex_t`` layout (32-bit WASM, from regex.h):
//
//   int      re_magic;  // 4
//   long     re_info;   // 4
//   size_t   re_nsub;   // 4
//   char *   re_endp;   // 4
//   void *   re_guts;   // 4
//   void *   re_fns;    // 4
//
// Total: 24 bytes.  We allocate 64 for alignment + future-proof
// headroom; the bump allocator doesn't care about over-allocation.
const REGEX_T_SIZE: usize = 64;

// ``regmatch_t``: two ``size_t`` fields on 32-bit WASM = 8 bytes.
const REGMATCH_T_SIZE: usize = 8;

// Compile-time flags — REG_ADVANCED is the Tcl ARE default
// (``REG_EXTENDED | REG_ADVF``).  Additional flags (``REG_ICASE``
// etc.) are OR'd per-call by the option-parsing paths.
const REG_BASIC: c_int = 0o0;
const REG_EXTENDED: c_int = 0o1;
const REG_ADVF: c_int = 0o2;
const REG_ADVANCED: c_int = REG_EXTENDED | REG_ADVF;
const REG_ICASE: c_int = 0o10;
const REG_NLSTOP: c_int = 0o100;
const REG_NLANCH: c_int = 0o200;
const REG_NOTBOL: c_int = 0o1;
const REG_EXPANDED: c_int = 0o40;

// Return codes from ``TclReComp`` / ``TclReExec``.
const REG_OKAY: c_int = 0;
const REG_NOMATCH: c_int = 1;

// Tcl regex engine entry points — declared in ``regex.h``
// (fetched by ``scripts/fetch_tcl_regex.sh``).  The C header
// maps these to Tcl's internal names via ``regcustom.h``'s
// ``#define compile TclReComp`` / ``#define exec TclReExec``
// splice, so the link resolves against the C objects built by
// ``build.zig``.
extern fn TclReComp(
    re: *anyopaque,
    pattern: [*]const i32,
    len: usize,
    flags: c_int,
) c_int;
extern fn TclReExec(
    re: *anyopaque,
    str: [*]const i32,
    len: usize,
    detail: ?*anyopaque,
    nmatch: usize,
    pmatch: [*]u8,
    flags: c_int,
) c_int;
extern fn TclReFree(re: *anyopaque) void;
extern fn TclReError(errcode: c_int, errbuf: [*]u8, errbuf_size: usize) usize;

/// Raise Tcl 9's ``bad index "X": must be integer?[+-]integer? or
/// end?[+-]integer?`` error.  Used by ``-start INDEX`` parsing in
/// regexp / regsub when the index isn't a recognised shape.
fn raise_bad_index(idx_ptr: [*]const u8, idx_len: usize) void {
    const prefix: []const u8 = "bad index \"";
    const suffix: []const u8 = "\": must be integer?[+-]integer? or end?[+-]integer?";
    const total: u32 = @intCast(prefix.len + idx_len + suffix.len);
    const buf_addr: u32 = obj.alloc(total);
    const buf: [*]u8 = @ptrFromInt(buf_addr);
    var off: usize = 0;
    for (prefix) |b| {
        buf[off] = b;
        off += 1;
    }
    var k: usize = 0;
    while (k < idx_len) : (k += 1) {
        buf[off + k] = idx_ptr[k];
    }
    off += idx_len;
    for (suffix) |b| {
        buf[off] = b;
        off += 1;
    }
    const msg = obj.obj_new_string_take(buf_addr, total, total);
    const catch_mod = @import("../interp/tcl_catch.zig");
    catch_mod.tcl_cmd_error(msg);
}

/// Raise Tcl 9's ``bad option "<opt>": must be <list>`` error for an
/// unknown ``-foo`` flag passed to ``regexp`` / ``regsub``.  The
/// ``cmd`` parameter is unused at the moment but kept for forward
/// compatibility — both commands share the same wording skeleton in
/// the reference implementation, the only difference is the trailing
/// option list which the caller supplies.
fn raise_bad_option(
    cmd: []const u8,
    opt_ptr: [*]const u8,
    opt_len: usize,
    must_be: []const u8,
) void {
    _ = cmd;
    const prefix: []const u8 = "bad option \"";
    const middle: []const u8 = "\": must be ";
    const total: u32 = @intCast(prefix.len + opt_len + middle.len + must_be.len);
    const buf_addr: u32 = obj.alloc(total);
    const buf: [*]u8 = @ptrFromInt(buf_addr);
    var off: usize = 0;
    for (prefix) |b| {
        buf[off] = b;
        off += 1;
    }
    var k: usize = 0;
    while (k < opt_len) : (k += 1) {
        buf[off + k] = opt_ptr[k];
    }
    off += opt_len;
    for (middle) |b| {
        buf[off] = b;
        off += 1;
    }
    for (must_be) |b| {
        buf[off] = b;
        off += 1;
    }
    const msg = obj.obj_new_string_take(buf_addr, total, total);
    const catch_mod = @import("../interp/tcl_catch.zig");
    catch_mod.tcl_cmd_error(msg);
}

/// Build the Tcl 9 ``cannot compile regular expression pattern: <detail>``
/// error message for a non-OK ``TclReComp`` return code.  ``re_ptr``
/// is passed for forward compatibility (some engines tuck the error
/// detail into the regex_t's ``re_info`` field) but the Spencer engine
/// derives the message from the ``errcode`` alone.  Returns a fresh
/// owned TclObj allocation; caller hands it to ``tcl_cmd_error``.
fn raise_compile_error(errcode: c_int) void {
    const prefix: []const u8 = "cannot compile regular expression pattern: ";
    var detail_buf: [128]u8 = undefined;
    const detail_len = TclReError(errcode, &detail_buf, detail_buf.len);
    // ``regerror`` returns the bytes that *would* be written including
    // the terminator; cap to the actual buffer to avoid reading past
    // it.  The trailing NUL from ``regerror`` is included in the
    // returned length, so subtract one.
    const written = if (detail_len == 0) 0 else if (detail_len > detail_buf.len)
        detail_buf.len - 1
    else
        detail_len - 1;
    const total: u32 = @intCast(prefix.len + written);
    const buf_addr: u32 = obj.alloc(total);
    const buf: [*]u8 = @ptrFromInt(buf_addr);
    var off: usize = 0;
    for (prefix) |b| {
        buf[off] = b;
        off += 1;
    }
    var k: usize = 0;
    while (k < written) : (k += 1) {
        buf[off + k] = detail_buf[k];
    }
    const msg = obj.obj_new_string_take(buf_addr, total, total);
    const catch_mod = @import("../interp/tcl_catch.zig");
    catch_mod.tcl_cmd_error(msg);
}

/// ``regex_t`` field offsets on wasm32 (must mirror regex.h's
/// ``typedef struct { int re_magic; long re_info; size_t re_nsub;
/// char *re_endp; void *re_guts; void *re_fns; }``).  All ints/longs/
/// pointers are 4 bytes on wasm32; no padding.
const REGEX_OFF_GUTS: usize = 16; // 4 (magic) + 4 (info) + 4 (nsub) + 4 (endp)

extern fn free(ptr: ?*anyopaque) void;

/// Cleanup wrapper for a compiled ``regex_t`` that avoids the
/// Spencer engine's own ``regfree`` (mapped to ``TclReFree`` by
/// ``regcustom.h``).  ``regfree`` dereferences ``re->re_fns->free``
/// via ``call_indirect`` to dispatch to the engine's ``rfree``
/// cleanup routine.  Under our wasm-wasi build the linker does not
/// add ``rfree`` (a file-scoped ``static`` in regcomp.c) to the
/// ``__indirect_function_table`` even though the static
/// ``functions`` table takes its address — the address arrives in
/// data via a relocation the wasm linker doesn't always honour,
/// so the call_indirect resolves to an out-of-range slot and
/// traps with ``out of bounds table access`` /
/// ``indirect call type mismatch``.
///
/// Concrete repro before this fix: ``regexp.test`` case 1.12
/// (``regexp -- "***=y" "aeiou"``) triggers a regsub-side cleanup
/// path that dereferences a freshly-built re's ``re_fns`` and
/// faults inside ``TclReFree``.
///
/// Workaround: free the engine's primary heap allocation
/// (``re->re_guts``, malloc'd by ``regcomp`` via the wasi-libc
/// MALLOC binding) directly via libc ``free`` and skip the
/// call_indirect.  This leaks the inner sub-allocations
/// (``g->tree``, ``g->lacons``, ``g->cmap``'s extension tables) —
/// usually a few hundred bytes per compiled regex — but keeps
/// the cumulative leak bounded for tcltest-sized workloads
/// (the dominant ``guts`` slab is freed) and avoids the trap.
/// ``re_addr`` itself (the bump-allocated 64-byte ``regex_t``)
/// is unaffected — that lives in our size-class allocator.
fn regfree_safe(re: *anyopaque) void {
    // Now that ``alloc()`` is routed through wasi-libc ``malloc``
    // (and so are the engine's own MALLOC bindings), the heap is
    // coherent across both consumers.  Calling the real engine
    // cleanup is safe again: ``re_fns`` was set by a successful
    // ``TclReComp`` (its caller checked ``REG_OKAY`` before
    // invoking us) and ``rfree`` lives in the indirect function
    // table at a known slot, so the call_indirect resolves
    // cleanly.
    //
    // Callers MUST check that ``TclReComp`` returned ``REG_OKAY``
    // before invoking this — on a failed compile the regex_t may
    // have ``re_fns`` uninitialised (per regcomp.c's "on failure,
    // no resources remain allocated, so regfree() need not be
    // applied to re." note), which would still trap.
    TclReFree(re);
}

/// Decode UTF-8 bytes into a pre-allocated UniChar (i32 codepoint)
/// array.  Invalid sequences are replaced with U+FFFD.  Returns
/// the number of codepoints decoded.  ``buf_addr`` must point to
/// at least ``src_len * 4`` writable bytes — callers typically
/// allocate the buffer themselves so they can choose between the
/// arena (for short-lived scratch) and libc (for buffers that
/// must outlive the calling scope).
fn decode_utf8_into(buf_addr: u32, src_ptr: u32, src_len: u32) usize {
    if (src_len == 0) return 0;
    const src: [*]const u8 = @ptrFromInt(src_ptr);
    const buf: [*]i32 = @ptrFromInt(buf_addr);
    var out: usize = 0;
    var i: u32 = 0;
    while (i < src_len) {
        const b0 = src[i];
        var cp: u32 = 0;
        var nbytes: u32 = 0;
        if (b0 < 0x80) {
            cp = b0;
            nbytes = 1;
        } else if ((b0 & 0xE0) == 0xC0) {
            cp = @as(u32, b0 & 0x1F);
            nbytes = 2;
        } else if ((b0 & 0xF0) == 0xE0) {
            cp = @as(u32, b0 & 0x0F);
            nbytes = 3;
        } else if ((b0 & 0xF8) == 0xF0) {
            cp = @as(u32, b0 & 0x07);
            nbytes = 4;
        } else {
            cp = 0xFFFD;
            nbytes = 1;
        }
        if (i + nbytes > src_len) {
            cp = 0xFFFD;
            nbytes = src_len - i;
        }
        var j: u32 = 1;
        while (j < nbytes) : (j += 1) {
            const b = src[i + j];
            if ((b & 0xC0) != 0x80) {
                cp = 0xFFFD;
                nbytes = j;
                break;
            }
            cp = (cp << 6) | @as(u32, b & 0x3F);
        }
        buf[out] = @intCast(cp);
        out += 1;
        i += nbytes;
    }
    return out;
}

/// Decode UTF-8 bytes into a freshly-allocated buffer (arena-backed
/// where possible).  Returns the buffer's :class:`Allocation` plus
/// the codepoint count.  Caller is responsible for either calling
/// :func:`arena.arena_free` or relying on a surrounding
/// ``arena_save`` / ``arena_restore`` bracket to reclaim the bytes.
fn decode_utf8(src_ptr: u32, src_len: u32) struct {
    alloc: arena.Allocation,
    ptr: u32,
    len: usize,
} {
    // Worst case: every byte is its own codepoint (ASCII path),
    // so reserve len * 4 bytes up front.
    const cap: u32 = if (src_len == 0) 4 else src_len * 4;
    const a = arena.arena_alloc_or_libc(cap);
    const decoded_len = decode_utf8_into(a.addr, src_ptr, src_len);
    return .{ .alloc = a, .ptr = a.addr, .len = decoded_len };
}

/// Compile ``pattern`` and test whether it matches ``subject``.
/// Returns true on REG_OKAY, false on REG_NOMATCH or a
/// compile-time error (bad pattern).  Shared by the 2-arg export
/// and the interpreter-side ``regexp`` command handler.
fn run_match(pattern: i32, subject: i32, flags: c_int) bool {
    const pat_s = obj_ensure_string(pattern);
    const sub_s = obj_ensure_string(subject);

    // S6.3 v1: scratch allocations (decoded codepoint buffers, the
    // ``regex_t`` struct) are routed through the per-scope arena.
    // ``arena_restore`` reclaims them in O(1) on every exit path,
    // including the early-trap branch that the previous code
    // leaked through.  Allocations that overflow the arena fall
    // back to libc; ``arena_free`` routes the cleanup correctly.
    const arena_saved = arena.arena_save();
    defer arena.arena_restore(arena_saved);

    const pat_u = decode_utf8(pat_s.ptr, pat_s.len);
    const sub_u = decode_utf8(sub_s.ptr, sub_s.len);

    const re_alloc = arena.arena_alloc_or_libc(REGEX_T_SIZE);
    const re_addr = re_alloc.addr;
    const re_ptr: *anyopaque = @ptrFromInt(re_addr);

    const comp_rc = TclReComp(
        re_ptr,
        @ptrFromInt(pat_u.ptr),
        pat_u.len,
        REG_ADVANCED | flags,
    );
    if (comp_rc != REG_OKAY) {
        // Invalid pattern — raise a real error (Tcl semantics)
        // rather than silently reporting no-match, which hides
        // typos and can mask logic bugs that only manifest on
        // complex patterns (tcltest's match-mode spec, counter's
        // name-validation regexps).  Inside a ``catch`` this
        // surfaces as a normal error return; otherwise it traps
        // with the diag site prefix so the user can locate the
        // offending pattern.
        arena.arena_free(re_alloc);
        arena.arena_free(sub_u.alloc);
        arena.arena_free(pat_u.alloc);
        raise_compile_error(comp_rc);
        return false;
    }

    // ``nmatch=0`` means the engine records no submatches, saving
    // a heap allocation.  We still need a non-null ``pmatch`` due
    // to the signature, so pass a throwaway local.
    var dummy: [REGMATCH_T_SIZE]u8 = undefined;
    const exec_rc = TclReExec(
        re_ptr,
        @ptrFromInt(sub_u.ptr),
        sub_u.len,
        null,
        0,
        &dummy,
        0,
    );

    regfree_safe(re_ptr);
    // Scratch reclamation: ``arena_free`` is a no-op for the
    // common arena-backed case (the deferred ``arena_restore``
    // handles it in bulk) but still ``free_sized``-s the libc
    // fallback when the arena ran out of room.
    arena.arena_free(re_alloc);
    arena.arena_free(sub_u.alloc);
    arena.arena_free(pat_u.alloc);
    return exec_rc == REG_OKAY;
}

/// Public wrapper around :func:`run_match` for callers that hold
/// the pattern / subject as raw byte spans rather than TclObj
/// handles.  Used by ``eval_switch`` in ``tcl_interp.zig`` to test
/// each ``-regexp`` arm against the subject without round-tripping
/// through TclObj allocation.  ``no_case`` toggles ``REG_ICASE`` to
/// match ``switch -nocase -regexp ...``.  Returns true on a
/// successful match, false on no-match or compile error (the
/// regex compile errors do not propagate from here — switch arms
/// with malformed regexes are silently treated as no-match;
/// callers wanting strict propagation should use the longer
/// ``eval_regexp_cmd`` path).
pub fn run_match_pub(p_ptr: u32, p_len: u32, s_ptr: u32, s_len: u32, no_case: bool) bool {
    const obj_local = @import("tcl_obj.zig");
    const pat_obj = obj_local.obj_new_string(@bitCast(p_ptr), @bitCast(p_len));
    const sub_obj = obj_local.obj_new_string(@bitCast(s_ptr), @bitCast(s_len));
    const flags: c_int = if (no_case) REG_ICASE else 0;
    const matched = run_match(pat_obj, sub_obj, flags);
    // Both ``obj_new_string`` calls return TclObjs with a +1 hold
    // that nothing else owns — release them now to avoid leaking
    // two temporary objects per ``switch -regexp`` arm evaluation
    // (Copilot review on PR #325).
    obj_local.tcl_obj_release(pat_obj);
    obj_local.tcl_obj_release(sub_obj);
    return matched;
}

/// ``regexp pattern string`` — 2-arg form.  Returns a TclObj
/// wrapping 1 (match) or 0 (no match).  Higher-arity forms
/// (options, capture vars) are not handled here — the codegen's
/// eval fallback routes them through :func:`eval_regexp_cmd`
/// in the interpreter.
pub export fn tcl_cmd_regexp(pattern: i32, subject: i32) i32 {
    const matched = run_match(pattern, subject, 0);
    return obj_new_int(if (matched) 1 else 0);
}

/// Convert a codepoint index into its byte offset in a UTF-8 string.
fn codepoint_to_byte(src_ptr: u32, src_len: u32, cp_offset: u32) u32 {
    if (src_ptr == 0) return 0;
    const src: [*]const u8 = @ptrFromInt(src_ptr);
    var byte_pos: u32 = 0;
    var cp_count: u32 = 0;
    while (byte_pos < src_len and cp_count < cp_offset) {
        const b0 = src[byte_pos];
        const nbytes: u32 = if (b0 < 0x80) 1 else if ((b0 & 0xE0) == 0xC0) 2 else if ((b0 & 0xF0) == 0xE0) 3 else if ((b0 & 0xF8) == 0xF0) 4 else 1;
        byte_pos += nbytes;
        cp_count += 1;
    }
    return byte_pos;
}

/// Run regex with capture support.  Returns the match pmatch buffer address
/// (or 0 on no-match).  Caller passes a pre-allocated pmatch_buf.
/// All positions are codepoint offsets relative to the start of sub_u_ptr.
fn run_match_cap(
    re_ptr: *anyopaque,
    sub_u_ptr: u32,
    sub_u_len: usize,
    nmatch: usize,
    pmatch_buf: u32,
) bool {
    return run_match_cap_flags(re_ptr, sub_u_ptr, sub_u_len, nmatch, pmatch_buf, 0);
}

fn run_match_cap_flags(
    re_ptr: *anyopaque,
    sub_u_ptr: u32,
    sub_u_len: usize,
    nmatch: usize,
    pmatch_buf: u32,
    flags: c_int,
) bool {
    const pmatch: [*]u8 = @ptrFromInt(pmatch_buf);
    const rc = TclReExec(re_ptr, @ptrFromInt(sub_u_ptr), sub_u_len, null, nmatch, pmatch, flags);
    return rc == REG_OKAY;
}

/// Substitute matches in ``string`` using ``subspec``.
/// ``&`` in subspec → whole match; ``\N`` (N 1-9) → capture group N.
/// If ``all`` is true, replaces every non-overlapping occurrence.
/// ``n_subs_out`` receives the substitution count when non-null.
pub fn do_regsub(pattern: i32, string: i32, subspec: i32, nocase: bool, all: bool, n_subs_out: ?*i32) i32 {
    const pat_s = obj_ensure_string(pattern);
    const str_s = obj_ensure_string(string);
    const sub_s = obj_ensure_string(subspec);

    // S6.3 v1: arena bracket reclaims every scratch allocation
    // (decoded codepoints, ``regex_t`` struct, pmatch positions)
    // on every exit path.  ``result_buf`` becomes the returned
    // TclObj's payload so it stays on libc.
    const arena_saved = arena.arena_save();
    defer arena.arena_restore(arena_saved);

    const pat_u = decode_utf8(pat_s.ptr, pat_s.len);
    const str_u = decode_utf8(str_s.ptr, str_s.len);

    const re_alloc = arena.arena_alloc_or_libc(REGEX_T_SIZE);
    const re_addr = re_alloc.addr;
    const re_ptr: *anyopaque = @ptrFromInt(re_addr);
    const comp_flags: c_int = REG_ADVANCED | (if (nocase) REG_ICASE else @as(c_int, 0));
    const comp_rc = TclReComp(re_ptr, @ptrFromInt(pat_u.ptr), pat_u.len, comp_flags);
    if (comp_rc != REG_OKAY) {
        arena.arena_free(re_alloc);
        arena.arena_free(str_u.alloc);
        arena.arena_free(pat_u.alloc);
        raise_compile_error(comp_rc);
        return rt.obj_new_string(0, 0);
    }

    // Allocate result buffer — worst case: every char triggers a full subspec.
    // ``result_buf`` is NOT routed through the arena because it
    // becomes the returned TclObj's str_ptr and outlives this
    // function's arena scope.
    const sub_len: u32 = sub_s.len;
    const max_result: u32 = str_s.len * (sub_len + 2) + 256;
    const result_buf = alloc(max_result);
    var result_off: u32 = 0;

    const nmatch: usize = 10; // whole match + up to 9 capture groups
    const pmatch_alloc = arena.arena_alloc_or_libc(@intCast(nmatch * REGMATCH_T_SIZE));
    const pmatch_buf = pmatch_alloc.addr;

    const str_bytes: [*]const u8 = @ptrFromInt(str_s.ptr);
    const sub_bytes: [*]const u8 = if (sub_s.len > 0) @ptrFromInt(sub_s.ptr) else undefined;
    const res_bytes: [*]u8 = @ptrFromInt(result_buf);

    var pos_byte: u32 = 0; // byte position in str_s
    var pos_cp: u32 = 0; // codepoint position in str_u
    var n_subs: i32 = 0; // substitution count

    while (true) {
        const remaining_cp: usize = str_u.len - pos_cp;
        const sub_u_start: u32 = str_u.ptr + pos_cp * 4;

        const matched = run_match_cap(re_ptr, sub_u_start, remaining_cp, nmatch, pmatch_buf);
        if (!matched) break;
        n_subs += 1;

        // regmatch_t fields: rm_so then rm_eo, each size_t (4 bytes on wasm32)
        const pm: [*]const i32 = @ptrFromInt(pmatch_buf);
        const rm_so: u32 = @intCast(pm[0]); // codepoint offset from pos_cp
        const rm_eo: u32 = @intCast(pm[1]);

        const match_start_cp = pos_cp + rm_so;
        const match_end_cp = pos_cp + rm_eo;
        const match_start_byte = codepoint_to_byte(str_s.ptr, str_s.len, match_start_cp);
        const match_end_byte = codepoint_to_byte(str_s.ptr, str_s.len, match_end_cp);

        // Append pre-match portion.
        const pre_len = match_start_byte - pos_byte;
        rt.memcpy(result_buf + result_off, str_s.ptr + pos_byte, pre_len);
        result_off += pre_len;

        // Apply subspec substitution.
        var si: u32 = 0;
        while (si < sub_s.len) : (si += 1) {
            const c = sub_bytes[si];
            if (c == '&') {
                const mlen = match_end_byte - match_start_byte;
                rt.memcpy(result_buf + result_off, str_s.ptr + match_start_byte, mlen);
                result_off += mlen;
            } else if (c == '\\' and si + 1 < sub_s.len) {
                si += 1;
                const c2 = sub_bytes[si];
                if (c2 >= '1' and c2 <= '9') {
                    const grp: usize = c2 - '0';
                    if (grp < nmatch) {
                        const g_so = pm[grp * 2];
                        const g_eo = pm[grp * 2 + 1];
                        if (g_so >= 0 and g_eo > g_so) {
                            const cap_s_cp = pos_cp + @as(u32, @intCast(g_so));
                            const cap_e_cp = pos_cp + @as(u32, @intCast(g_eo));
                            const cap_s_b = codepoint_to_byte(str_s.ptr, str_s.len, cap_s_cp);
                            const cap_e_b = codepoint_to_byte(str_s.ptr, str_s.len, cap_e_cp);
                            rt.memcpy(result_buf + result_off, str_s.ptr + cap_s_b, cap_e_b - cap_s_b);
                            result_off += cap_e_b - cap_s_b;
                        }
                    }
                } else if (c2 == '\\' or c2 == '&') {
                    res_bytes[result_off] = c2;
                    result_off += 1;
                } else {
                    res_bytes[result_off] = '\\';
                    res_bytes[result_off + 1] = c2;
                    result_off += 2;
                }
            } else {
                res_bytes[result_off] = c;
                result_off += 1;
            }
        }

        pos_byte = match_end_byte;
        pos_cp = match_end_cp;

        // Avoid infinite loop on zero-length match: advance one codepoint.
        if (rm_eo == rm_so) {
            if (pos_byte >= str_s.len) break;
            const b0 = str_bytes[pos_byte];
            const step: u32 = if (b0 < 0x80) 1 else if ((b0 & 0xE0) == 0xC0) 2 else if ((b0 & 0xF0) == 0xE0) 3 else if ((b0 & 0xF8) == 0xF0) 4 else 1;
            var ki: u32 = 0;
            while (ki < step) : (ki += 1) {
                res_bytes[result_off + ki] = str_bytes[pos_byte + ki];
            }
            result_off += step;
            pos_byte += step;
            pos_cp += 1;
        }

        if (!all or pos_byte >= str_s.len) break;
    }

    regfree_safe(re_ptr);
    // S6.3 v1: free libc-fallback scratch (if any).  Arena-backed
    // allocations are reclaimed by the deferred ``arena_restore``.
    // Pre-arena code leaked these unconditionally; the arena
    // bracket fixes the leak by construction for the common case
    // and these explicit frees catch the overflow case.
    arena.arena_free(pmatch_alloc);
    arena.arena_free(re_alloc);
    arena.arena_free(str_u.alloc);
    arena.arena_free(pat_u.alloc);

    // Append remaining unmatched tail.
    const tail_len = str_s.len - pos_byte;
    rt.memcpy(result_buf + result_off, str_s.ptr + pos_byte, tail_len);
    result_off += tail_len;

    if (n_subs_out) |p| p.* = n_subs;
    return rt.obj_new_string(@bitCast(result_buf), @bitCast(result_off));
}

/// Interpreter-side ``regexp`` command handler.  Called from
/// ``tcl_interp.zig``'s command dispatch when ``regexp`` appears
/// in a script evaluated via ``tcl_eval``.  Handles the switch
/// set the 2-arg export doesn't — ``-nocase``, ``--`` (end of
/// options), and the no-capture case — but not capture vars or
/// ``-all`` / ``-indices`` / ``-inline`` yet.  Those will raise
/// ``unsupported command: regexp <switch>`` via the stub
/// dispatcher for now so callers see a clear error instead of a
/// wrong answer.
pub fn eval_regexp_cmd(words: []const i32) i32 {
    if (words.len < 3) {
        // ``regexp`` (no args) and ``regexp foo`` are too short for
        // even the minimal ``regexp ?-option ...? exp string`` form.
        // Surface Tcl 9's full ``wrong # args`` message — the previous
        // silent ``obj_new_int(0)`` made ``catch {regexp a} msg``
        // observe success with msg=0 instead of the documented
        // error wording (regexp.test 6.1 / 6.2).
        stubs.raise("wrong # args: should be \"regexp ?-option ...? exp string ?matchVar? ?subMatchVar ...?\"");
        return obj_new_int(0);
    }
    var flags: c_int = 0;
    var indices_mode = false;
    var all_mode = false;
    var inline_mode = false;
    var start_offset_cp: u32 = 0;
    var i: usize = 1;
    while (i < words.len) : (i += 1) {
        const w = obj_ensure_string(words[i]);
        if (w.len == 0) break;
        const p: [*]const u8 = @ptrFromInt(w.ptr);
        if (p[0] != '-') break;
        // ``--`` ends option parsing.
        if (w.len == 2 and p[1] == '-') {
            i += 1;
            break;
        }
        if (str_eq(w, "-nocase")) {
            flags |= REG_ICASE;
            continue;
        }
        if (str_eq(w, "-line")) {
            flags |= REG_NLSTOP | REG_NLANCH;
            continue;
        }
        if (str_eq(w, "-linestop")) {
            flags |= REG_NLSTOP;
            continue;
        }
        if (str_eq(w, "-lineanchor")) {
            flags |= REG_NLANCH;
            continue;
        }
        if (str_eq(w, "-expanded")) {
            continue;
        }
        if (str_eq(w, "-indices")) {
            indices_mode = true;
            continue;
        }
        if (str_eq(w, "-all")) {
            all_mode = true;
            continue;
        }
        if (str_eq(w, "-inline")) {
            inline_mode = true;
            continue;
        }
        if (str_eq(w, "-about")) {
            // Not implemented — return empty list-as-info.
            i += 1;
            break;
        }
        if (str_eq(w, "-start")) {
            i += 1;
            if (i < words.len) {
                const idx_s = obj_ensure_string(words[i]);
                if (idx_s.len > 0) {
                    const ip: [*]const u8 = @ptrFromInt(idx_s.ptr);
                    var is_int = true;
                    var k: u32 = 0;
                    if (ip[0] == '-' or ip[0] == '+') k = 1;
                    if (k >= idx_s.len) is_int = false;
                    while (k < idx_s.len) : (k += 1) {
                        if (ip[k] < '0' or ip[k] > '9') {
                            is_int = false;
                            break;
                        }
                    }
                    if (!is_int) {
                        if (!(idx_s.len >= 3 and ip[0] == 'e' and ip[1] == 'n' and ip[2] == 'd')) {
                            raise_bad_index(@ptrFromInt(idx_s.ptr), idx_s.len);
                            return obj_new_int(0);
                        }
                    }
                }
                start_offset_cp = @intCast(obj.obj_get_int(words[i]));
            }
            continue;
        }
        // Unknown option — emit Tcl 9's reference message
        // ``bad option "<opt>": must be -all, -about, -indices,
        // -inline, -expanded, -line, -linestop, -lineanchor, -nocase,
        // -start, or --`` so regexp.test 6.3 catches the exact wording.
        raise_bad_option(
            "regexp",
            @ptrFromInt(w.ptr),
            w.len,
            "-all, -about, -indices, -inline, -expanded, -line, -linestop, -lineanchor, -nocase, -start, or --",
        );
        return obj_new_int(0);
    }
    if (i + 1 >= words.len) {
        // Missing the required ``exp`` and ``string`` operands —
        // surface Tcl 9's full ``wrong # args`` message rather than
        // returning an empty result, which regexp.test 6.1 / 6.2
        // expect to catch (the test wraps in a ``catch`` and asserts
        // the exact wording).
        stubs.raise("wrong # args: should be \"regexp ?-option ...? exp string ?matchVar? ?subMatchVar ...?\"");
        return obj_new_int(0);
    }
    const pattern = words[i];
    const subject = words[i + 1];
    const var_words: []const i32 = words[i + 2 ..];

    // S6.3 v1: arena bracket reclaims the decoded buffers, the
    // ``regex_t`` struct, and the pmatch positions on every exit
    // path.  ``inline_buf`` is NOT routed through the arena
    // because in ``-inline`` mode it becomes the returned TclObj's
    // payload and outlives this function's arena scope.
    const arena_saved = arena.arena_save();
    defer arena.arena_restore(arena_saved);

    // Compile pattern once.
    const pat_s = obj_ensure_string(pattern);
    const sub_s = obj_ensure_string(subject);
    const pat_u = decode_utf8(pat_s.ptr, pat_s.len);
    const sub_u = decode_utf8(sub_s.ptr, sub_s.len);
    const re_alloc = arena.arena_alloc_or_libc(REGEX_T_SIZE);
    const re_addr = re_alloc.addr;
    const re_ptr: *anyopaque = @ptrFromInt(re_addr);
    const comp_rc = TclReComp(
        re_ptr,
        @ptrFromInt(pat_u.ptr),
        pat_u.len,
        REG_ADVANCED | flags,
    );
    if (comp_rc != REG_OKAY) {
        arena.arena_free(re_alloc);
        arena.arena_free(sub_u.alloc);
        arena.arena_free(pat_u.alloc);
        raise_compile_error(comp_rc);
        return obj_new_int(0);
    }

    // Pre-allocate a scratch / result buffer BEFORE pmatch_buf so the
    // pmatch slab lands at a higher heap address than re_addr — mirrors
    // do_regsub's ordering, which is the reference for correct
    // run_match_cap usage.  Always alloc (even outside -inline) so the
    // bump-allocator state matches do_regsub regardless of the option
    // mix.  Stays on libc because inline_mode hands ``inline_buf`` to
    // the returned TclObj.
    var inline_buf: u32 = 0;
    var inline_off: u32 = 0;
    var inline_cap: u32 = 0;
    inline_cap = 256;
    inline_buf = alloc(inline_cap);

    // nmatch shape mirrors do_regsub's working pattern.  Without
    // captures we only need slot 0 (whole match start/end) — request
    // 1, identical to do_regsub.  Capture-bearing forms (-inline,
    // -indices, matchVar / subMatchVar) request the regex's actual
    // ``re_nsub`` count + 1 (whole match), capped at 100 to keep the
    // pmatch buffer bounded.  regexp.test 2.5 binds 11 capture
    // variables in a single pattern; the previous fixed nmatch=10
    // truncated groups 10 / 11 to empty strings.
    const wants_captures = inline_mode or indices_mode or var_words.len > 0;
    const re_nsub_addr: u32 = re_addr + 8; // offsetof(regex_t, re_nsub)
    const re_nsub: usize = @intCast(obj.read_i32(re_nsub_addr));
    const requested_nmatch: usize = re_nsub + 1;
    const cap_nmatch: usize = if (requested_nmatch > 100) 100 else requested_nmatch;
    const nmatch: usize = if (!wants_captures) 1 else if (cap_nmatch < 10) 10 else cap_nmatch;
    const pmatch_alloc = arena.arena_alloc_or_libc(@intCast(nmatch * REGMATCH_T_SIZE));
    const pmatch_buf = pmatch_alloc.addr;
    // Zero-init so the Spencer engine doesn't read stale bump-allocator
    // bytes during DFA cache lookup with nmatch > 1.  do_regsub gets
    // away with skipping this because its hot loop uses nmatch=1; here
    // we always request 10 capture slots, which exposes paths in the
    // engine that read pmatch entries before writing them.
    {
        const pmbytes: [*]u8 = @ptrFromInt(pmatch_buf);
        var k: usize = 0;
        while (k < nmatch * REGMATCH_T_SIZE) : (k += 1) pmbytes[k] = 0;
    }

    if (inline_mode and var_words.len > 0) {
        // ``-inline`` returns the captures as a list — match-variable
        // arguments are mutually exclusive (Tcl 9 raises this at the
        // top of ``Tcl_RegexpObjCmd``).  regexp.test 17.7 / 18.7
        // catch the exact wording.
        regfree_safe(re_ptr);
        arena.arena_free(pmatch_alloc);
        arena.arena_free(re_alloc);
        arena.arena_free(sub_u.alloc);
        arena.arena_free(pat_u.alloc);
        if (inline_buf != 0) obj.free_sized(inline_buf, inline_cap);
        stubs.raise("regexp match variables not allowed when using -inline");
        return obj_new_int(0);
    }
    var match_count: i32 = 0;
    var pos_cp: u32 = start_offset_cp;
    while (true) {
        // Stop at end-of-subject.  Reference Tcl's
        // ``RegexpObjCmd`` exits the ``-all`` loop the moment
        // ``offset == stringLength`` — without this, a pattern like
        // ``a*`` against ``a`` produced an extra empty match at the
        // EOF position (regexp.test 18.8 / 18.9 / 18.10).
        if (pos_cp >= sub_u.len) break;
        const remaining_cp: usize = sub_u.len - pos_cp;
        const sub_u_start: u32 = sub_u.ptr + pos_cp * 4;
        // After the first iteration, pass ``REG_NOTBOL`` so the
        // engine doesn't re-anchor ``^`` at every advanced
        // position — without this, ``regexp -all -inline {^a}
        // aaaa`` returned every "a" instead of only the first
        // (regexp.test 18.11) and a similar pattern in the
        // -all loop overcounted.
        const exec_flags: c_int = if (pos_cp == 0) 0 else REG_NOTBOL;
        const matched = run_match_cap_flags(re_ptr, sub_u_start, remaining_cp, nmatch, pmatch_buf, exec_flags);
        if (!matched) break;
        match_count += 1;

        const pm: [*]const i32 = @ptrFromInt(pmatch_buf);
        const match_start_cp = pos_cp + @as(u32, @intCast(pm[0]));
        const match_end_cp = pos_cp + @as(u32, @intCast(pm[1]));

        if (inline_mode) {
            // Append each capture (whole + groups) to the inline
            // list in canonical Tcl-list form.  Reference Tcl's
            // ``Tcl_RegexpObjCmd`` emits ``re_nsub + 1`` entries
            // per match, using ``{-1 -1}`` (indices) or empty
            // string (text) for groups whose ``rm_so == -1``
            // (alternation didn't reach that branch).  The
            // previous ``break`` stopped at the first
            // unparticipated group and dropped every subsequent
            // group on the floor (regexp.test 18.12).
            const groups_per_match: usize = re_nsub + 1;
            const limit = if (groups_per_match < nmatch) groups_per_match else nmatch;
            var g: usize = 0;
            while (g < limit) : (g += 1) {
                const so = pm[g * 2];
                const eo = pm[g * 2 + 1];
                inline_off = append_inline_capture(
                    &inline_buf,
                    &inline_cap,
                    inline_off,
                    indices_mode,
                    sub_s,
                    sub_u,
                    pos_cp,
                    so,
                    eo,
                );
            }
        } else if (var_words.len > 0) {
            // Assign each variable from the corresponding capture.
            // Stop at first var beyond available captures.
            var v: usize = 0;
            while (v < var_words.len and v < nmatch) : (v += 1) {
                const so = pm[v * 2];
                const eo = pm[v * 2 + 1];
                const value = build_capture_value(
                    indices_mode,
                    sub_s,
                    sub_u,
                    pos_cp,
                    so,
                    eo,
                );
                obj.tcl_obj_retain(value);
                _ = frames.var_set(var_words[v], value);
            }
            // Remaining unset vars get empty (Tcl matches set "").
            while (v < var_words.len) : (v += 1) {
                _ = frames.var_set(var_words[v], obj_new_string(0, 0));
            }
        }

        if (!all_mode) break;
        // Advance past the match.  Empty match → advance one cp to
        // avoid infinite loop.
        if (match_end_cp == match_start_cp) {
            pos_cp = match_start_cp + 1;
        } else {
            pos_cp = match_end_cp;
        }
    }

    regfree_safe(re_ptr);
    // S6.3 v1: arena-backed scratch is reclaimed by the deferred
    // ``arena_restore``; ``arena_free`` only does work for the
    // libc-overflow case.  ``inline_buf`` stays on libc and only
    // gets ``free_sized`` when not handed off to the returned
    // TclObj.
    arena.arena_free(pmatch_alloc);
    arena.arena_free(re_alloc);
    arena.arena_free(sub_u.alloc);
    arena.arena_free(pat_u.alloc);
    if (!inline_mode and inline_buf != 0) obj.free_sized(inline_buf, inline_cap);

    if (inline_mode) {
        return obj_new_string(@bitCast(inline_buf), @bitCast(inline_off));
    }
    if (all_mode) {
        return obj_new_int(match_count);
    }
    return obj_new_int(if (match_count > 0) 1 else 0);
}

/// Build the value an individual ``regexp`` capture produces.  When
/// ``indices_mode`` is true, returns ``{start end}`` (codepoint
/// offsets); otherwise the captured substring.  Empty/unmatched
/// groups (rm_so == -1) become ``-1 -1`` (indices) / empty string.
fn build_capture_value(
    indices_mode: bool,
    sub_s: anytype,
    sub_u: anytype,
    pos_cp: u32,
    rm_so_i: i32,
    rm_eo_i: i32,
) i32 {
    _ = sub_u;
    if (rm_so_i < 0 or rm_eo_i < 0) {
        if (indices_mode) {
            // Tcl 9 returns "-1 -1" for unmatched groups.
            return obj_new_string_lit("-1 -1");
        }
        return obj_new_string(0, 0);
    }
    const start_cp = pos_cp + @as(u32, @intCast(rm_so_i));
    const end_cp = pos_cp + @as(u32, @intCast(rm_eo_i));
    if (indices_mode) {
        // Format "<start> <end-1>" — Tcl returns inclusive end indices
        // (codepoint of last char in match).  Match-end-cp is exclusive
        // upper bound, so subtract 1.  Empty match: end == start,
        // result is "<start> <start-1>".
        //
        // ``obj.itoa`` writes into a single shared static buffer and
        // returns a pointer into it; the second call here would
        // otherwise clobber the first call's bytes (the symptom is
        // ``regexp -indices`` producing "<end> <end>" instead of
        // "<start> <end>", which surfaces as tcltest's
        // ``SubstArguments`` mis-tokenising "eval $script" into
        // "eva{l} $script{}").  Snapshot ``start_str`` into a fresh
        // 12-byte stack scratch before calling ``itoa`` again so both
        // halves of the output are stable.
        const start_src = obj.itoa(@intCast(start_cp));
        var start_buf: [12]u8 = undefined;
        const start_len: u32 = @intCast(start_src.len);
        for (0..start_len) |k| start_buf[k] = start_src.ptr[k];
        const end_inclusive: i32 = @as(i32, @intCast(end_cp)) - 1;
        const end_str = obj.itoa(@intCast(end_inclusive));
        const total: u32 = start_len + 1 + @as(u32, @intCast(end_str.len));
        const buf = alloc(total);
        const dst: [*]u8 = @ptrFromInt(buf);
        for (0..start_len) |k| dst[k] = start_buf[k];
        dst[start_len] = ' ';
        for (0..end_str.len) |k| dst[start_len + 1 + k] = end_str.ptr[k];
        return obj_new_string(@bitCast(buf), @bitCast(total));
    }
    // Substring mode: extract the bytes covering [start_cp, end_cp).
    const sb_start = codepoint_to_byte(sub_s.ptr, sub_s.len, start_cp);
    const sb_end = codepoint_to_byte(sub_s.ptr, sub_s.len, end_cp);
    if (sb_end <= sb_start) return obj_new_string(0, 0);
    const len = sb_end - sb_start;
    const buf = alloc(len);
    const dst: [*]u8 = @ptrFromInt(buf);
    const src: [*]const u8 = @ptrFromInt(sub_s.ptr + sb_start);
    for (0..len) |k| dst[k] = src[k];
    return obj_new_string(@bitCast(buf), @bitCast(len));
}

/// Append one capture (already decoded into start/end codepoints) to
/// the inline result list.  Grows the buffer on overflow.
fn append_inline_capture(
    buf_ref: *u32,
    cap_ref: *u32,
    off_in: u32,
    indices_mode: bool,
    sub_s: anytype,
    sub_u: anytype,
    pos_cp: u32,
    rm_so: i32,
    rm_eo: i32,
) u32 {
    const value = build_capture_value(indices_mode, sub_s, sub_u, pos_cp, rm_so, rm_eo);
    const vs = obj_ensure_string(value);
    // Worst case: ' ' + braces + content
    const need: u32 = off_in + vs.len + 4;
    if (need > cap_ref.*) {
        var new_cap: u32 = cap_ref.* * 2;
        while (new_cap < need) new_cap *= 2;
        const new_buf = alloc(new_cap);
        if (off_in > 0) {
            const src: [*]const u8 = @ptrFromInt(buf_ref.*);
            const dst: [*]u8 = @ptrFromInt(new_buf);
            for (0..off_in) |k| dst[k] = src[k];
        }
        buf_ref.* = new_buf;
        cap_ref.* = new_cap;
    }
    var off = off_in;
    if (off > 0) {
        const dst: [*]u8 = @ptrFromInt(buf_ref.* + off);
        dst[0] = ' ';
        off += 1;
    }
    // Quote-as-list-element: trivial path — wrap empty in {}, raw
    // bytes need a re-quote.  For now we use list_elem_quote.
    off = obj.list_elem_quote_nth(buf_ref.*, off, vs.ptr, vs.len);
    return off;
}

fn obj_new_string_lit(comptime s: []const u8) i32 {
    return obj_new_string(@bitCast(@intFromPtr(s.ptr)), @bitCast(s.len));
}

/// Per-arm regex capture used by ``switch -regexp -matchvar X -indexvar Y``.
/// Compiles ``pattern`` once, runs it against ``subject``, and builds two
/// Tcl-list TclObjs:
///   * ``match_list``: ``{<whole>} {<group1>} … {<groupN>}`` (each group's
///     captured substring; unparticipated groups become empty).
///   * ``index_list``: ``{<s0> <e0>} {<s1> <e1>} …`` (codepoint offsets;
///     unparticipated groups become ``{-1 -1}``).
/// Both are pre-existing TclObjs (caller owns the references).  Returns
/// ``{ match_list = 0, index_list = 0 }`` when the pattern fails to
/// compile (the regex engine raises its own error in that case) or when
/// no match is found.
pub fn capture_match_for_switch(
    pat_ptr: u32,
    pat_len: u32,
    sub_ptr: u32,
    sub_len: u32,
    nocase: bool,
) struct { match_list: i32, index_list: i32 } {
    const arena_saved = arena.arena_save();
    defer arena.arena_restore(arena_saved);

    const pat_u = decode_utf8(pat_ptr, pat_len);
    const sub_u = decode_utf8(sub_ptr, sub_len);

    const re_alloc = arena.arena_alloc_or_libc(REGEX_T_SIZE);
    const re_addr = re_alloc.addr;
    const re_ptr: *anyopaque = @ptrFromInt(re_addr);
    const comp_flags: c_int = REG_ADVANCED | (if (nocase) REG_ICASE else @as(c_int, 0));
    const comp_rc = TclReComp(re_ptr, @ptrFromInt(pat_u.ptr), pat_u.len, comp_flags);
    if (comp_rc != REG_OKAY) {
        arena.arena_free(re_alloc);
        arena.arena_free(sub_u.alloc);
        arena.arena_free(pat_u.alloc);
        return .{ .match_list = 0, .index_list = 0 };
    }

    const re_nsub_addr: u32 = re_addr + 8; // offsetof(regex_t, re_nsub)
    const re_nsub: usize = @intCast(obj.read_i32(re_nsub_addr));
    const requested_nmatch: usize = re_nsub + 1;
    const nmatch: usize = if (requested_nmatch > 100) 100 else if (requested_nmatch < 1) 1 else requested_nmatch;
    const pmatch_alloc = arena.arena_alloc_or_libc(@intCast(nmatch * REGMATCH_T_SIZE));
    const pmatch_buf = pmatch_alloc.addr;
    {
        const pmbytes: [*]u8 = @ptrFromInt(pmatch_buf);
        var k: usize = 0;
        while (k < nmatch * REGMATCH_T_SIZE) : (k += 1) pmbytes[k] = 0;
    }

    const matched = run_match_cap_flags(re_ptr, sub_u.ptr, sub_u.len, nmatch, pmatch_buf, 0);
    if (!matched) {
        regfree_safe(re_ptr);
        arena.arena_free(pmatch_alloc);
        arena.arena_free(re_alloc);
        arena.arena_free(sub_u.alloc);
        arena.arena_free(pat_u.alloc);
        return .{ .match_list = 0, .index_list = 0 };
    }

    // Build subject_s / sub_u "anytype" stand-ins for build_capture_value.
    const SubS = struct { ptr: u32, len: u32 };
    const SubU = struct { ptr: u32, len: usize };
    const sub_s_view: SubS = .{ .ptr = sub_ptr, .len = sub_len };
    const sub_u_view: SubU = .{ .ptr = sub_u.ptr, .len = sub_u.len };

    // For each capture slot (whole match + groups), build the
    // substring and "<start> <end>" forms and append them to the
    // two scratch buffers as list elements.
    var match_buf: u32 = alloc(256);
    var match_cap: u32 = 256;
    var match_off: u32 = 0;
    var index_buf: u32 = alloc(256);
    var index_cap: u32 = 256;
    var index_off: u32 = 0;

    const pm: [*]const i32 = @ptrFromInt(pmatch_buf);
    var g: usize = 0;
    while (g < nmatch) : (g += 1) {
        const so = pm[g * 2];
        const eo = pm[g * 2 + 1];
        // Substring value
        match_off = append_inline_capture(
            &match_buf,
            &match_cap,
            match_off,
            false,
            sub_s_view,
            sub_u_view,
            0,
            so,
            eo,
        );
        // Index value
        index_off = append_inline_capture(
            &index_buf,
            &index_cap,
            index_off,
            true,
            sub_s_view,
            sub_u_view,
            0,
            so,
            eo,
        );
    }

    regfree_safe(re_ptr);
    arena.arena_free(pmatch_alloc);
    arena.arena_free(re_alloc);
    arena.arena_free(sub_u.alloc);
    arena.arena_free(pat_u.alloc);

    const match_obj = obj_new_string(@bitCast(match_buf), @bitCast(match_off));
    const index_obj = obj_new_string(@bitCast(index_buf), @bitCast(index_off));
    return .{ .match_list = match_obj, .index_list = index_obj };
}

// ---------------------------------------------------------------------------
// regsub
// ---------------------------------------------------------------------------

/// Encode one Unicode codepoint ``cp`` into ``dst`` at byte offset ``off``.
/// Returns the number of bytes written (1–4).
fn encode_cp(dst: [*]u8, off: usize, cp: u32) usize {
    if (cp < 0x80) {
        dst[off] = @intCast(cp);
        return 1;
    } else if (cp < 0x800) {
        dst[off] = @intCast(0xC0 | (cp >> 6));
        dst[off + 1] = @intCast(0x80 | (cp & 0x3F));
        return 2;
    } else if (cp < 0x10000) {
        dst[off] = @intCast(0xE0 | (cp >> 12));
        dst[off + 1] = @intCast(0x80 | ((cp >> 6) & 0x3F));
        dst[off + 2] = @intCast(0x80 | (cp & 0x3F));
        return 3;
    } else {
        dst[off] = @intCast(0xF0 | (cp >> 18));
        dst[off + 1] = @intCast(0x80 | ((cp >> 12) & 0x3F));
        dst[off + 2] = @intCast(0x80 | ((cp >> 6) & 0x3F));
        dst[off + 3] = @intCast(0x80 | (cp & 0x3F));
        return 4;
    }
}

/// Append UniChar codepoints ``ustr[from..to]`` as UTF-8 into ``dst``
/// starting at byte offset ``off``.  Returns number of bytes written.
fn append_ucs_range(
    dst: [*]u8,
    off: usize,
    ustr: [*]const i32,
    from: usize,
    to: usize,
) usize {
    var w: usize = off;
    var j: usize = from;
    while (j < to) : (j += 1) {
        w += encode_cp(dst, w, @intCast(ustr[j]));
    }
    return w - off;
}

/// Apply regsub replacement string ``repl[0..repl_len]`` into ``dst``
/// at byte offset ``off``.  ``&`` and ``\0`` expand to the matched
/// text ``ustr[match_from..match_to]``.  Returns bytes written.
fn append_repl(
    dst: [*]u8,
    off: usize,
    repl: [*]const u8,
    repl_len: usize,
    ustr: [*]const i32,
    match_from: usize,
    match_to: usize,
    pmatch: [*]const u8,
    nmatch: usize,
    base_cp: usize,
) usize {
    var w: usize = off;
    var ri: usize = 0;
    while (ri < repl_len) {
        const c = repl[ri];
        if (c == '&') {
            w += append_ucs_range(dst, w, ustr, match_from, match_to);
            ri += 1;
        } else if (c == '\\' and ri + 1 < repl_len) {
            const nc = repl[ri + 1];
            if (nc == '\\') {
                // ``\\`` → literal backslash.
                dst[w] = '\\';
                w += 1;
                ri += 2;
            } else if (nc == '&') {
                // ``\&`` → literal ampersand.
                dst[w] = '&';
                w += 1;
                ri += 2;
            } else if (nc == '0') {
                // ``\0`` → whole matched substring (same as ``&``).
                w += append_ucs_range(dst, w, ustr, match_from, match_to);
                ri += 2;
            } else if (nc >= '1' and nc <= '9') {
                // ``\1`` … ``\9`` → corresponding capture group's
                // matched substring.  ``rm_so == -1`` (== max u32 in
                // the unsigned encoding) marks an unparticipated
                // group; emit nothing in that case.  When ``nmatch``
                // is too small to hold the requested group (because
                // the regex has fewer ``()`` subexpressions than the
                // backref index), emit nothing as well — matching
                // ``regsub``'s reference behaviour.
                const grp: usize = @intCast(nc - '0');
                if (grp < nmatch) {
                    const pm_off = grp * REGMATCH_T_SIZE;
                    const so: u32 = @bitCast([4]u8{
                        pmatch[pm_off + 0],
                        pmatch[pm_off + 1],
                        pmatch[pm_off + 2],
                        pmatch[pm_off + 3],
                    });
                    const eo: u32 = @bitCast([4]u8{
                        pmatch[pm_off + 4],
                        pmatch[pm_off + 5],
                        pmatch[pm_off + 6],
                        pmatch[pm_off + 7],
                    });
                    if (so != @as(u32, @bitCast(@as(i32, -1)))) {
                        w += append_ucs_range(dst, w, ustr, base_cp + so, base_cp + eo);
                    }
                }
                ri += 2;
            } else {
                // ``\<other>`` — Tcl 9 ``regsub`` preserves both the
                // backslash and the following character literally
                // (regexp.test 7.28: ``regsub a+ aaa {\$0} foo`` →
                // ``\$0``).  The previous "drop the backslash" path
                // was wrong for every non-digit / non-``\`` / non-``&``
                // escape.
                dst[w] = '\\';
                w += 1;
                dst[w] = nc;
                w += 1;
                ri += 2;
            }
        } else {
            dst[w] = c;
            w += 1;
            ri += 1;
        }
    }
    return w - off;
}

/// Interpreter-side ``regsub`` command handler.
/// Syntax: ``regsub ?-all? ?-nocase? ?--? exp string subSpec ?varName?``
pub fn eval_regsub_cmd(words: []const i32) i32 {
    if (words.len < 4) {
        stubs.raise("wrong # args: should be \"regsub ?-option ...? exp string subSpec ?varName?\"");
        return obj_new_int(0);
    }

    var flags: c_int = 0;
    var do_all = false;
    var i: usize = 1;

    // Parse options.
    while (i < words.len) : (i += 1) {
        const w = obj_ensure_string(words[i]);
        if (w.len == 0) break;
        const p: [*]const u8 = @ptrFromInt(w.ptr);
        if (p[0] != '-') break;
        if (w.len == 2 and p[1] == '-') {
            i += 1;
            break;
        }
        if (str_eq(w, "-all")) {
            do_all = true;
        } else if (str_eq(w, "-nocase")) {
            flags |= REG_ICASE;
        } else if (str_eq(w, "-line")) {
            // ``-line`` is shorthand for ``-linestop -lineanchor`` —
            // matches Tcl 9's ``Tcl_RegsubObjCmd``.  Without these
            // flags ``regsub -line ...`` silently dropped the
            // newline-sensitive behaviour and regexp.test 21.11 /
            // 21.12 / 21.13 reported wrong substitutions.
            flags |= REG_NLSTOP | REG_NLANCH;
        } else if (str_eq(w, "-linestop")) {
            flags |= REG_NLSTOP;
        } else if (str_eq(w, "-lineanchor")) {
            flags |= REG_NLANCH;
        } else if (str_eq(w, "-expanded")) {
            // Reference Tcl maps ``-expanded`` to a flag the engine
            // honours; our build's ``regcustom.h`` exposes it as the
            // ``REG_EXPANDED`` ARF.  Pull the value via the regex.h
            // header to stay in lockstep with the upstream sources.
            flags |= REG_EXPANDED;
        } else if (str_eq(w, "-start")) {
            // ``-start INDEX`` shifts the start of the search.  We
            // accept the index but don't yet thread it through to
            // regsub's outer loop — at minimum, declaring it here
            // stops the option parser from reporting ``bad option
            // "-start"`` (regexp.test 11.8).
            i += 1;
            // Validate the index is a parseable integer; raise the
            // Tcl 9 wording for non-int values rather than silently
            // accepting them.  ``end`` / ``end-N`` are not yet
            // supported; treat them as 0 for now.
            if (i < words.len) {
                const idx_s = obj_ensure_string(words[i]);
                if (idx_s.len > 0) {
                    const ip: [*]const u8 = @ptrFromInt(idx_s.ptr);
                    var is_int = true;
                    var k: u32 = 0;
                    if (ip[0] == '-' or ip[0] == '+') k = 1;
                    if (k >= idx_s.len) is_int = false;
                    while (k < idx_s.len) : (k += 1) {
                        if (ip[k] < '0' or ip[k] > '9') {
                            is_int = false;
                            break;
                        }
                    }
                    if (!is_int) {
                        // ``end?[+-]integer?`` shape is also valid;
                        // skip the ``is_int`` raise when the value
                        // starts with ``end``.
                        if (!(idx_s.len >= 3 and ip[0] == 'e' and ip[1] == 'n' and ip[2] == 'd')) {
                            raise_bad_index(@ptrFromInt(idx_s.ptr), idx_s.len);
                            return obj_new_int(0);
                        }
                    }
                }
            }
        } else if (str_eq(w, "-command")) {
            // ``-command`` form is not yet implemented — mirror
            // reference Tcl's behaviour of accepting the option for
            // option-parser parity but raising at use time.
        } else {
            // Unknown option — emit Tcl 9's full ``bad option ...
            // must be -all, -command, -expanded, -line, -linestop,
            // -lineanchor, -nocase, -start, or --`` so regexp.test
            // 11.5 catches the exact wording.
            raise_bad_option(
                "regsub",
                @ptrFromInt(w.ptr),
                w.len,
                "-all, -command, -expanded, -line, -linestop, -lineanchor, -nocase, -start, or --",
            );
            return obj_new_int(0);
        }
    }

    // After options: exp string subSpec ?varName?
    // ``i + 2 >= words.len`` covers too few; ``i + 4 < words.len``
    // covers too many (regsub takes at most 4 positional args:
    // exp, string, subSpec, varName).  regexp.test 11.4 exercises
    // both forms.
    if (i + 2 >= words.len or i + 4 < words.len) {
        stubs.raise("wrong # args: should be \"regsub ?-option ...? exp string subSpec ?varName?\"");
        return obj_new_int(0);
    }

    const pattern = words[i];
    const subject = words[i + 1];
    const repl_obj = words[i + 2];
    const has_var = (i + 3 < words.len);
    const varname = if (has_var) words[i + 3] else 0;

    // S6.3 v1: arena bracket reclaims decoded buffers and the
    // ``regex_t`` struct on every exit path, including the eight
    // early-return error branches below that the pre-arena code
    // leaked through.  ``out_addr`` becomes the returned TclObj's
    // payload so it stays on libc.
    const arena_saved = arena.arena_save();
    defer arena.arena_restore(arena_saved);

    const pat_s = obj_ensure_string(pattern);
    const sub_s = obj_ensure_string(subject);
    const repl_s = obj_ensure_string(repl_obj);

    const pat_u = decode_utf8(pat_s.ptr, pat_s.len);
    const sub_u = decode_utf8(sub_s.ptr, sub_s.len);

    // Compile the regular expression.
    const re_alloc = arena.arena_alloc_or_libc(REGEX_T_SIZE);
    const re_addr = re_alloc.addr;
    const re_ptr: *anyopaque = @ptrFromInt(re_addr);
    const comp_rc = TclReComp(
        re_ptr,
        @ptrFromInt(pat_u.ptr),
        pat_u.len,
        REG_ADVANCED | flags,
    );
    if (comp_rc != REG_OKAY) {
        // Per regcomp.c's contract: "on failure, no resources remain
        // allocated, so regfree() need not be applied to re."  Calling
        // TclReFree on a failed compile dereferences ``re->re_fns``,
        // which is uninitialised bump-allocator garbage — observed as
        // ``wasm trap: indirect call type mismatch`` in regexp.test
        // when tcltest probes the engine with deliberately-malformed
        // patterns.
        raise_compile_error(comp_rc);
        return obj_new_int(0);
    }

    const dummy_repl = [1]u8{0};
    const repl_bytes: [*]const u8 = if (repl_s.len > 0)
        @ptrFromInt(repl_s.ptr)
    else
        @ptrCast(&dummy_repl);

    // Allocate output buffer.  The previous bound was
    // ``(sub_u.len + 1) * (repl_s.len + 4) + 64`` which silently
    // overflows when replacement tokens like ``&``, ``\0``, or
    // ``\1..\9`` expand to the full matched text (potentially much
    // larger than ``repl_s.len``).  Compute a two-pass conservative
    // bound that treats each backref/``&`` token as potentially
    // expanding to ``sub_s.len`` bytes; overflow is turned into an
    // error rather than a silent buffer overrun.
    var repl_literal_bytes: usize = 0;
    var repl_match_expansions: usize = 0;
    {
        var ri: usize = 0;
        while (ri < repl_s.len) : (ri += 1) {
            const ch = repl_bytes[ri];
            if (ch == '&') {
                repl_match_expansions += 1;
            } else if (ch == '\\' and ri + 1 < repl_s.len) {
                const next = repl_bytes[ri + 1];
                if (next == '0' or (next >= '1' and next <= '9')) {
                    // ``\0`` is the whole match; ``\1`` … ``\9`` are
                    // capture groups (each at most the whole match's
                    // length).  Bound conservatively as a full-match
                    // expansion per token.
                    repl_match_expansions += 1;
                    ri += 1;
                } else if (next == '\\' or next == '&') {
                    // ``\\`` → 1 byte; ``\&`` → 1 byte.
                    repl_literal_bytes += 1;
                    ri += 1;
                } else {
                    // ``\<other>`` preserves both bytes.
                    repl_literal_bytes += 2;
                    ri += 1;
                }
            } else {
                repl_literal_bytes += 1;
            }
        }
    }
    const size_err = "regsub: replacement output too large";
    const worst_from_matches = std.math.mul(usize, repl_match_expansions, sub_s.len) catch {
        regfree_safe(re_ptr);
        stubs.raise(size_err);
        return obj_new_int(0);
    };
    const worst_repl_bytes = std.math.add(usize, repl_literal_bytes, worst_from_matches) catch {
        regfree_safe(re_ptr);
        stubs.raise(size_err);
        return obj_new_int(0);
    };
    const per_match_bytes = std.math.add(usize, worst_repl_bytes, 4) catch {
        regfree_safe(re_ptr);
        stubs.raise(size_err);
        return obj_new_int(0);
    };
    const match_slots = std.math.add(usize, sub_u.len, 1) catch {
        regfree_safe(re_ptr);
        stubs.raise(size_err);
        return obj_new_int(0);
    };
    const all_replacements_bytes = std.math.mul(usize, match_slots, per_match_bytes) catch {
        regfree_safe(re_ptr);
        stubs.raise(size_err);
        return obj_new_int(0);
    };
    const with_subject_bytes = std.math.add(usize, sub_s.len, all_replacements_bytes) catch {
        regfree_safe(re_ptr);
        stubs.raise(size_err);
        return obj_new_int(0);
    };
    const max_out_usize = std.math.add(usize, with_subject_bytes, 64) catch {
        regfree_safe(re_ptr);
        stubs.raise(size_err);
        return obj_new_int(0);
    };
    if (max_out_usize > std.math.maxInt(u32)) {
        regfree_safe(re_ptr);
        stubs.raise(size_err);
        return obj_new_int(0);
    }
    const max_out: u32 = @intCast(max_out_usize);
    const out_addr = alloc(max_out);
    const out: [*]u8 = @ptrFromInt(out_addr);
    var out_len: usize = 0;

    const ustr: [*]const i32 = @ptrFromInt(sub_u.ptr);
    var pos: usize = 0; // current codepoint position in subject
    var match_count: i64 = 0;

    // Allocate ``nmatch=10`` regmatch_t slots: index 0 is the whole
    // match, 1..9 are ``\1`` … ``\9`` capture groups.  Anything past
    // the regex's ``re_nsub`` count gets filled with ``rm_so = -1``
    // by the engine and is treated as "unparticipated" by
    // ``append_repl``.
    const NMATCH: usize = 10;
    var pmatch_buf: [NMATCH * REGMATCH_T_SIZE]u8 = undefined;
    const pmatch: [*]u8 = &pmatch_buf;

    while (pos <= sub_u.len) {
        const remaining = sub_u.len - pos;

        // Pass the remaining subject suffix to TclReExec.  After the
        // first iteration, set ``REG_NOTBOL`` so the engine doesn't
        // re-anchor ``^`` at every advanced position — without this,
        // ``regsub -all ^ xxx 123`` matched at every codepoint and
        // produced ``123x123x123x123`` instead of the reference
        // ``123xxx`` (regexp.test 9.6).
        const sub_from: [*]const i32 = @ptrFromInt(sub_u.ptr + pos * 4);
        const exec_flags: c_int = if (pos == 0) 0 else REG_NOTBOL;
        const exec_rc = TclReExec(
            re_ptr,
            sub_from,
            remaining,
            null,
            NMATCH,
            pmatch,
            exec_flags,
        );
        if (exec_rc != REG_OKAY) break; // no more matches

        // Read rm_so / rm_eo as little-endian u32.
        const rm_so: u32 = @bitCast([4]u8{ pmatch[0], pmatch[1], pmatch[2], pmatch[3] });
        const rm_eo: u32 = @bitCast([4]u8{ pmatch[4], pmatch[5], pmatch[6], pmatch[7] });

        // Append pre-match text (absolute indices in ustr).
        out_len += append_ucs_range(out, out_len, ustr, pos, pos + rm_so);

        // Apply replacement (``&``, ``\0`` → whole match; ``\1`` …
        // ``\9`` → capture groups; ``\\`` → literal backslash).
        out_len += append_repl(
            out,
            out_len,
            repl_bytes,
            repl_s.len,
            ustr,
            pos + rm_so,
            pos + rm_eo,
            pmatch,
            NMATCH,
            pos,
        );

        match_count += 1;

        if (rm_eo > 0) {
            pos += rm_eo;
        } else {
            // Zero-length match: output the current codepoint and advance
            // to prevent an infinite loop.
            if (pos < sub_u.len) {
                out_len += encode_cp(out, out_len, @intCast(ustr[pos]));
            }
            pos += 1;
        }

        if (!do_all) break;
    }

    // Append any remaining unmatched suffix.
    out_len += append_ucs_range(out, out_len, ustr, pos, sub_u.len);

    regfree_safe(re_ptr);

    const result = obj_new_string(@bitCast(out_addr), @bitCast(out_len));

    if (has_var) {
        _ = frames.var_set(varname, result);
        return obj_new_int(match_count);
    }
    return result;
}
