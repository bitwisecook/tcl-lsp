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
const rt = @import("tcl_runtime.zig");
const stubs = @import("tcl_stubs.zig");
const frames = @import("tcl_frames.zig");
const obj_ensure_string = rt.obj_ensure_string;
const obj_new_int = rt.obj_new_int;
const obj_new_string = rt.obj_new_string;
const alloc = rt.alloc;

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

/// Decode UTF-8 bytes into a fresh UniChar (i32 codepoint) array
/// on the bump allocator.  Invalid sequences are replaced with
/// U+FFFD.  Returns the buffer's WASM address + length in
/// codepoints.
fn decode_utf8(src_ptr: u32, src_len: u32) struct { ptr: u32, len: usize } {
    // Worst case: every byte is its own codepoint (ASCII path),
    // so reserve len * 4 bytes up front.  Over-allocation is
    // free in a bump allocator.
    const buf_addr = alloc(src_len * 4);
    if (src_len == 0) {
        return .{ .ptr = buf_addr, .len = 0 };
    }
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
    return .{ .ptr = buf_addr, .len = out };
}

/// Compile ``pattern`` and test whether it matches ``subject``.
/// Returns true on REG_OKAY, false on REG_NOMATCH or a
/// compile-time error (bad pattern).  Shared by the 2-arg export
/// and the interpreter-side ``regexp`` command handler.
fn run_match(pattern: i32, subject: i32, flags: c_int) bool {
    const pat_s = obj_ensure_string(pattern);
    const sub_s = obj_ensure_string(subject);

    const pat_u = decode_utf8(pat_s.ptr, pat_s.len);
    const sub_u = decode_utf8(sub_s.ptr, sub_s.len);

    const re_addr = alloc(REGEX_T_SIZE);
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
        stubs.raise("regexp: couldn't compile regular expression pattern");
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

    TclReFree(re_ptr);
    return exec_rc == REG_OKAY;
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
    const src: [*]const u8 = @ptrFromInt(src_ptr);
    var byte_pos: u32 = 0;
    var cp_count: u32 = 0;
    while (byte_pos < src_len and cp_count < cp_offset) {
        const b0 = src[byte_pos];
        const nbytes: u32 = if (b0 < 0x80) 1
            else if ((b0 & 0xE0) == 0xC0) 2
            else if ((b0 & 0xF0) == 0xE0) 3
            else if ((b0 & 0xF8) == 0xF0) 4
            else 1;
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
    const pmatch: [*]u8 = @ptrFromInt(pmatch_buf);
    const rc = TclReExec(re_ptr, @ptrFromInt(sub_u_ptr), sub_u_len, null, nmatch, pmatch, 0);
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

    const pat_u = decode_utf8(pat_s.ptr, pat_s.len);
    const str_u = decode_utf8(str_s.ptr, str_s.len);

    const re_addr = alloc(REGEX_T_SIZE);
    const re_ptr: *anyopaque = @ptrFromInt(re_addr);
    const comp_flags: c_int = REG_ADVANCED | (if (nocase) REG_ICASE else @as(c_int, 0));
    const comp_rc = TclReComp(re_ptr, @ptrFromInt(pat_u.ptr), pat_u.len, comp_flags);
    if (comp_rc != REG_OKAY) {
        stubs.raise("regsub: couldn't compile regular expression pattern");
        return rt.obj_new_string(0, 0);
    }

    // Allocate result buffer — worst case: every char triggers a full subspec.
    const sub_len: u32 = sub_s.len;
    const max_result: u32 = str_s.len * (sub_len + 2) + 256;
    const result_buf = alloc(max_result);
    var result_off: u32 = 0;

    const nmatch: usize = 10; // whole match + up to 9 capture groups
    const pmatch_buf = alloc(@intCast(nmatch * REGMATCH_T_SIZE));

    const str_bytes: [*]const u8 = @ptrFromInt(str_s.ptr);
    const sub_bytes: [*]const u8 = if (sub_s.len > 0) @ptrFromInt(sub_s.ptr) else undefined;
    const res_bytes: [*]u8 = @ptrFromInt(result_buf);

    var pos_byte: u32 = 0; // byte position in str_s
    var pos_cp:   u32 = 0; // codepoint position in str_u
    var n_subs:   i32 = 0; // substitution count

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

        const match_start_cp   = pos_cp + rm_so;
        const match_end_cp     = pos_cp + rm_eo;
        const match_start_byte = codepoint_to_byte(str_s.ptr, str_s.len, match_start_cp);
        const match_end_byte   = codepoint_to_byte(str_s.ptr, str_s.len, match_end_cp);

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
        pos_cp   = match_end_cp;

        // Avoid infinite loop on zero-length match: advance one codepoint.
        if (rm_eo == rm_so) {
            if (pos_byte >= str_s.len) break;
            const b0 = str_bytes[pos_byte];
            const step: u32 = if (b0 < 0x80) 1
                else if ((b0 & 0xE0) == 0xC0) 2
                else if ((b0 & 0xF0) == 0xE0) 3
                else if ((b0 & 0xF8) == 0xF0) 4
                else 1;
            var ki: u32 = 0;
            while (ki < step) : (ki += 1) {
                res_bytes[result_off + ki] = str_bytes[pos_byte + ki];
            }
            result_off += step;
            pos_byte   += step;
            pos_cp     += 1;
        }

        if (!all or pos_byte >= str_s.len) break;
    }

    TclReFree(re_ptr);

    // Append remaining unmatched tail.
    const tail_len = str_s.len - pos_byte;
    rt.memcpy(result_buf + result_off, str_s.ptr + pos_byte, tail_len);
    result_off += tail_len;

    if (n_subs_out) |p| p.* = n_subs;
    return rt.obj_new_string(@intCast(result_buf), @intCast(result_off));
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
        return obj_new_int(0);
    }
    var flags: c_int = 0;
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
        if (w.len == 7 and p[1] == 'n' and p[2] == 'o' and p[3] == 'c' and
            p[4] == 'a' and p[5] == 's' and p[6] == 'e')
        {
            flags |= REG_ICASE;
            continue;
        }
        // Unknown option — raise a real error rather than treating
        // the switch token as the pattern.  Silently falling
        // through would make ``regexp -indices {a+} aa`` match the
        // literal ``-indices`` against ``{a+}`` and return a
        // wrong boolean, giving callers no indication that a
        // valid-but-unimplemented switch was ignored.  Raising
        // here pushes the caller toward either supplying ``--``
        // before a literal ``-``-prefixed pattern or updating
        // this handler to support the switch.
        stubs.raise("regexp: unsupported or unknown option");
        return obj_new_int(0);
    }
    if (i + 1 >= words.len) {
        return obj_new_int(0);
    }
    const pattern = words[i];
    const subject = words[i + 1];
    // Ignore trailing matchVar / subMatchVar args — we don't
    // support capture yet.  They'll just not be set, which is
    // observable but closer to correct than trapping.
    const matched = run_match(pattern, subject, flags);
    return obj_new_int(if (matched) 1 else 0);
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
        dst[off]     = @intCast(0xC0 | (cp >> 6));
        dst[off + 1] = @intCast(0x80 | (cp & 0x3F));
        return 2;
    } else if (cp < 0x10000) {
        dst[off]     = @intCast(0xE0 | (cp >> 12));
        dst[off + 1] = @intCast(0x80 | ((cp >> 6) & 0x3F));
        dst[off + 2] = @intCast(0x80 | (cp & 0x3F));
        return 3;
    } else {
        dst[off]     = @intCast(0xF0 | (cp >> 18));
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
            if (nc == '0') {
                w += append_ucs_range(dst, w, ustr, match_from, match_to);
            } else {
                dst[w] = nc;
                w += 1;
            }
            ri += 2;
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
        stubs.raise("regsub: wrong # args: should be \"regsub ?switches? exp string subSpec ?varName?\"");
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
        if (w.len == 2 and p[1] == '-') { i += 1; break; }
        if (w.len == 4 and p[1] == 'a' and p[2] == 'l' and p[3] == 'l') {
            do_all = true;
        } else if (w.len == 7 and p[1] == 'n' and p[2] == 'o' and p[3] == 'c' and
                   p[4] == 'a' and p[5] == 's' and p[6] == 'e')
        {
            flags |= REG_ICASE;
        } else {
            stubs.raise("regsub: unsupported option");
            return obj_new_int(0);
        }
    }

    // After options: exp string subSpec ?varName?
    if (i + 2 >= words.len) {
        stubs.raise("regsub: wrong # args: should be \"regsub ?switches? exp string subSpec ?varName?\"");
        return obj_new_int(0);
    }

    const pattern  = words[i];
    const subject  = words[i + 1];
    const repl_obj = words[i + 2];
    const has_var  = (i + 3 < words.len);
    const varname  = if (has_var) words[i + 3] else 0;

    const pat_s  = obj_ensure_string(pattern);
    const sub_s  = obj_ensure_string(subject);
    const repl_s = obj_ensure_string(repl_obj);

    const pat_u = decode_utf8(pat_s.ptr, pat_s.len);
    const sub_u = decode_utf8(sub_s.ptr, sub_s.len);

    // Compile the regular expression.
    const re_addr = alloc(REGEX_T_SIZE);
    const re_ptr: *anyopaque = @ptrFromInt(re_addr);
    const comp_rc = TclReComp(
        re_ptr,
        @ptrFromInt(pat_u.ptr),
        pat_u.len,
        REG_ADVANCED | flags,
    );
    if (comp_rc != REG_OKAY) {
        TclReFree(re_ptr);
        stubs.raise("regexp: couldn't compile regular expression pattern");
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
                    repl_match_expansions += 1;
                    ri += 1;
                } else {
                    repl_literal_bytes += 1;
                    ri += 1;
                }
            } else {
                repl_literal_bytes += 1;
            }
        }
    }
    const size_err = "regsub: replacement output too large";
    const worst_from_matches = std.math.mul(usize, repl_match_expansions, sub_s.len) catch {
        TclReFree(re_ptr);
        stubs.raise(size_err);
        return obj_new_int(0);
    };
    const worst_repl_bytes = std.math.add(usize, repl_literal_bytes, worst_from_matches) catch {
        TclReFree(re_ptr);
        stubs.raise(size_err);
        return obj_new_int(0);
    };
    const per_match_bytes = std.math.add(usize, worst_repl_bytes, 4) catch {
        TclReFree(re_ptr);
        stubs.raise(size_err);
        return obj_new_int(0);
    };
    const match_slots = std.math.add(usize, sub_u.len, 1) catch {
        TclReFree(re_ptr);
        stubs.raise(size_err);
        return obj_new_int(0);
    };
    const all_replacements_bytes = std.math.mul(usize, match_slots, per_match_bytes) catch {
        TclReFree(re_ptr);
        stubs.raise(size_err);
        return obj_new_int(0);
    };
    const with_subject_bytes = std.math.add(usize, sub_s.len, all_replacements_bytes) catch {
        TclReFree(re_ptr);
        stubs.raise(size_err);
        return obj_new_int(0);
    };
    const max_out_usize = std.math.add(usize, with_subject_bytes, 64) catch {
        TclReFree(re_ptr);
        stubs.raise(size_err);
        return obj_new_int(0);
    };
    if (max_out_usize > std.math.maxInt(u32)) {
        TclReFree(re_ptr);
        stubs.raise(size_err);
        return obj_new_int(0);
    }
    const max_out: u32 = @intCast(max_out_usize);
    const out_addr = alloc(max_out);
    const out: [*]u8 = @ptrFromInt(out_addr);
    var out_len: usize = 0;

    const ustr: [*]const i32 = @ptrFromInt(sub_u.ptr);
    var pos: usize = 0;          // current codepoint position in subject
    var match_count: i64 = 0;

    // One regmatch_t = two size_t (u32 on 32-bit) = 8 bytes.
    var pmatch: [REGMATCH_T_SIZE]u8 = undefined;

    while (pos <= sub_u.len) {
        const remaining = sub_u.len - pos;

        // Pass the remaining subject suffix to TclReExec.
        const sub_from: [*]const i32 = @ptrFromInt(sub_u.ptr + pos * 4);
        const exec_rc = TclReExec(
            re_ptr,
            sub_from,
            remaining,
            null,
            1,
            &pmatch,
            0,
        );
        if (exec_rc != REG_OKAY) break; // no more matches

        // Read rm_so / rm_eo as little-endian u32.
        const rm_so: u32 = @bitCast([4]u8{ pmatch[0], pmatch[1], pmatch[2], pmatch[3] });
        const rm_eo: u32 = @bitCast([4]u8{ pmatch[4], pmatch[5], pmatch[6], pmatch[7] });

        // Append pre-match text (absolute indices in ustr).
        out_len += append_ucs_range(out, out_len, ustr, pos, pos + rm_so);

        // Apply replacement (``&`` → matched text).
        out_len += append_repl(
            out,
            out_len,
            repl_bytes,
            repl_s.len,
            ustr,
            pos + rm_so,
            pos + rm_eo,
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

    TclReFree(re_ptr);

    const result = obj_new_string(@intCast(out_addr), @intCast(out_len));

    if (has_var) {
        _ = frames.var_set(varname, result);
        return obj_new_int(match_count);
    }
    return result;
}
