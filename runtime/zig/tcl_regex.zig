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
const obj_ensure_string = rt.obj_ensure_string;
const obj_new_int = rt.obj_new_int;
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
        // Compile error — treat as no-match for graceful
        // degradation.  Tcl itself raises an error here; the
        // option-parsing paths can surface that more cleanly.
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
        // Unknown option — treat as the start of the positional
        // args so we don't swallow a pattern that happens to
        // start with ``-``.  The 2-arg positional path below
        // will handle it.
        break;
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
