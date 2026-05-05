// Tcl substitution engine — $var, [cmd], and \escape expansion.
//
// Extracted from tcl_interp.zig so that both the interpreter's word
// expander (subst_word) and the ``subst`` command handler can share
// one canonical implementation without importing the entire interpreter.

const rt     = @import("../tcl_runtime.zig");
const frames = @import("../interp/tcl_frames.zig");
const obj_mod = @import("../valtypes/tcl_obj.zig");
const arena = @import("../valtypes/tcl_arena.zig");
const tcl_array = @import("../valtypes/tcl_array.zig");
const catch_mod = @import("../interp/tcl_catch.zig");
const result_mod = @import("../interp/tcl_result.zig");

const alloc            = rt.alloc;
const memcpy           = rt.memcpy;
const read_i32         = obj_mod.read_i32;
const write_i32        = obj_mod.write_i32;
const obj_new_string   = rt.obj_new_string;
const obj_ensure_string = rt.obj_ensure_string;

/// Build an error TclObj from a literal byte slice and route it
/// through ``tcl_cmd_error``.  Used by :func:`subst_flagged` to
/// report ``missing close-brace for variable name`` /
/// ``missing close-bracket`` / ``extra characters after close-brace``
/// — the fixed-text errors that reference Tcl's ``Tcl_SubstObj`` /
/// ``ParseTokens`` raise for malformed substitutions.
fn raise_subst_error(text: []const u8) i32 {
    const len: u32 = @intCast(text.len);
    const buf_addr = alloc(len);
    if (buf_addr == 0) {
        catch_mod.tcl_cmd_error(0);
        return 0;
    }
    const buf: [*]u8 = @ptrFromInt(buf_addr);
    for (text, 0..) |c, k| buf[k] = c;
    const msg = obj_mod.obj_new_string_take(buf_addr, len, len);
    catch_mod.tcl_cmd_error(msg);
    return 0;
}

/// Expand ``$var`` / ``[cmd]`` / ``\x`` in a raw byte span, with each
/// substitution kind independently enabled.  ``do_vars`` / ``do_cmds``
/// / ``do_bs`` correspond to the ``-novariables`` / ``-nocommands``
/// / ``-nobackslashes`` flags of the ``subst`` command.
///
/// Uses a single-pass "pieces" strategy: each resolved substitution is
/// recorded as a (ptr, len) pair in a bump-allocated scratch buffer;
/// all pieces are concatenated in one final memcpy.  This avoids
/// double-evaluating side-effecting commands (``[incr x]`` must fire
/// exactly once) and avoids the resize hazard that two-pass approaches
/// suffer when a variable changes between the sizing and copying passes.
pub fn subst_flagged(
    wptr: u32,
    wlen: u32,
    do_vars: bool,
    do_cmds: bool,
    do_bs: bool,
) i32 {
    return subst_flagged_full(wptr, wlen, do_vars, do_cmds, do_bs, false);
}

/// Same as :func:`subst_flagged` but with explicit ``from_subst_cmd``
/// gating.  Reference Tcl's ``Tcl_SubstObj`` (the ``subst`` command's
/// implementation) catches ``TCL_BREAK`` / ``TCL_CONTINUE`` /
/// ``TCL_RETURN`` from each nested ``[cmd]`` and folds them into the
/// substituted text — *but* normal word substitution during script
/// evaluation does NOT: a ``[break]`` inside ``while {1} {puts [break]}``
/// must propagate to the enclosing ``while`` loop.  Pass ``true``
/// from the ``subst`` command path; pass ``false`` from
/// ``subst_word`` / expression substitution / recursive
/// ``$arr(idx)`` resolution so the flags surface to the caller.
pub fn subst_flagged_full(
    wptr: u32,
    wlen: u32,
    do_vars: bool,
    do_cmds: bool,
    do_bs: bool,
    from_subst_cmd: bool,
) i32 {
    if (wlen == 0) return obj_new_string(0, 0);
    // Defensive bound: any wlen this large is almost certainly a corrupted
    // Token (likely from the parse cache replaying a slab whose source
    // buffer was freed).  Without this guard the per-piece scratch sizing
    // below (wlen*8 / wlen*4) panics on u32 overflow in ReleaseSafe and
    // takes the whole WASM module down.  Returning the bytes verbatim
    // matches the no-substitution fast path two lines down and is strictly
    // safer than allocating ~64 MiB of scratch for a junk span.
    const MAX_SUBST_WLEN: u32 = 8 * 1024 * 1024;
    if (wlen > MAX_SUBST_WLEN) {
        return obj_new_string(@bitCast(wptr), @bitCast(wlen));
    }
    const src: [*]const u8 = @ptrFromInt(wptr);
    var has_dollar = false;
    var has_bracket = false;
    var has_backslash = false;
    for (0..wlen) |i| {
        if (src[i] == '$') has_dollar = true;
        if (src[i] == '[') has_bracket = true;
        if (src[i] == '\\') has_backslash = true;
    }
    if (!has_dollar and !has_bracket and (!do_bs or !has_backslash)) {
        return obj_new_string(@bitCast(wptr), @bitCast(wlen));
    }
    // S6.3: route the two per-piece scratch buffers and every
    // ``esc_ptr`` escape allocation through the arena.  The
    // ``arena_save`` / ``arena_restore`` bracket reclaims them
    // all in O(1) at function exit.  Allocations that overflow
    // the arena fall back to libc transparently — ``arena_free``
    // routes the cleanup correctly for both cases.
    const arena_saved = arena.arena_save();
    defer arena.arena_restore(arena_saved);

    const pieces_cap: u32 = @min(wlen *| 8, 64 * 1024 * 1024);
    const pieces_alloc = arena.arena_alloc_or_libc(pieces_cap);
    const pieces_buf = pieces_alloc.addr;
    if (pieces_buf == 0) {
        // OOM on the per-piece scratch.  Without it the rest of
        // this function would write through address 0 — surface as
        // a benign empty TclObj instead and let the caller see the
        // raw bytes via a future retry.
        return obj_new_string(0, 0);
    }
    var n_pieces: u32 = 0;
    var total_out: u32 = 0;
    var lit_start: u32 = 0;
    var lit_run: u32 = 0;

    // MM-B.6: retain every TclObj whose bytes we push into the
    // pieces buffer.  push_piece records (ptr, len) borrowed from
    // the source obj; if the source gets released between
    // push_piece and the final concat, the borrowed bytes go
    // stale.  The retained_objs scratch holds those refs until
    // we're done concatenating, then releases them.  Bound is
    // wlen $-substs / [bracket] subs, so wlen u32s of scratch
    // is the worst case.
    const retained_cap: u32 = @min(wlen *| 4, 32 * 1024 * 1024);
    const retained_alloc = arena.arena_alloc_or_libc(retained_cap);
    const retained_objs = retained_alloc.addr;
    if (retained_objs == 0) {
        arena.arena_free(pieces_alloc);
        return obj_new_string(0, 0);
    }
    var n_retained: u32 = 0;
    const flush_lit = struct {
        fn go(pb: u32, np: *u32, to: *u32, start: u32, run: *u32, base: u32) void {
            if (run.* == 0) return;
            const slot = pb + np.* * 8;
            write_i32(slot, @bitCast(base + start));
            write_i32(slot + 4, @bitCast(run.*));
            np.* += 1;
            to.* += run.*;
            run.* = 0;
        }
    }.go;
    const push_piece = struct {
        fn go(pb: u32, np: *u32, to: *u32, ptr: u32, len: u32) void {
            if (len == 0) return;
            const slot = pb + np.* * 8;
            write_i32(slot, @bitCast(ptr));
            write_i32(slot + 4, @bitCast(len));
            np.* += 1;
            to.* += len;
        }
    }.go;
    var i: u32 = 0;
    while (i < wlen) {
        if (do_vars and src[i] == '$' and i + 1 < wlen) {
            flush_lit(pieces_buf, &n_pieces, &total_out, lit_start, &lit_run, wptr);
            i += 1;
            const vstart = i;
            if (src[i] == '{') {
                i += 1;
                const vs = i;
                while (i < wlen and src[i] != '}') {
                    // ``\<X>`` inside ``${...}`` consumes two bytes
                    // verbatim — the brace scanner must skip the
                    // escape so a ``\}`` doesn't close early.
                    if (src[i] == '\\' and i + 1 < wlen) i += 2 else i += 1;
                }
                if (i >= wlen) {
                    // Unclosed ``${...}`` — reference Tcl raises
                    // ``missing close-brace for variable name``
                    // (parse-18.9 / 18.10 / 18.11 / 18.12).  Cleanup
                    // happens via the deferred ``arena_restore``;
                    // any retained source objs are released below
                    // before we return.
                    var rj: u32 = 0;
                    while (rj < n_retained) : (rj += 1) {
                        const r = read_i32(retained_objs + rj * 4);
                        if (r != 0) obj_mod.tcl_obj_release(r);
                    }
                    arena.arena_free(retained_alloc);
                    arena.arena_free(pieces_alloc);
                    return raise_subst_error("missing close-brace for variable name");
                }
                const ve = i;
                i += 1; // consume '}'
                const name_obj = obj_new_string(@bitCast(wptr + vs), @bitCast(ve - vs));
                const val = frames.var_resolve(name_obj);
                // Release the temp name TclObj immediately — its
                // bytes were borrowed from the source script and
                // the lookup machinery doesn't retain a reference.
                // Issue #303: each ``${var}`` substitution leaked
                // one obj header per evaluation and pushed long-
                // running scripts past the wasm32 4 GiB ceiling.
                obj_mod.tcl_obj_release(name_obj);
                if (val != 0) {
                    const sv = obj_ensure_string(val);
                    push_piece(pieces_buf, &n_pieces, &total_out, sv.ptr, sv.len);
                    obj_mod.tcl_obj_retain(val);
                    write_i32(retained_objs + n_retained * 4, val);
                    n_retained += 1;
                }
            } else {
                // Variable-name scanner: bareword chars + ``::``
                // namespace separator (Tcl 9: ``$::tcl_library`` /
                // ``$ns::var``).  ``::`` consumes two bytes as a unit
                // — a single ``:`` does NOT belong to a variable name
                // (so ``$x:foo`` resolves ``$x`` and leaves ``:foo``
                // as literal).
                while (i < wlen) {
                    const c = src[i];
                    if ((c >= 'a' and c <= 'z') or
                        (c >= 'A' and c <= 'Z') or
                        (c >= '0' and c <= '9') or c == '_')
                    {
                        i += 1;
                    } else if (c == ':' and i + 1 < wlen and src[i + 1] == ':') {
                        i += 2;
                    } else {
                        break;
                    }
                }
                const name_obj = obj_new_string(@bitCast(wptr + vstart), @bitCast(i - vstart));
                // Array-element form ``$arr(idx)``: when the next byte
                // is ``(`` we consume up to the matching ``)``,
                // recursively substitute the index span (so
                // ``$arr($k)`` and ``$arr([f])`` work), and look the
                // element up with ``array_get``.  Without this branch
                // the parser falls out of the variable-name loop at
                // the ``(`` and emits ``arr`` plus the literal
                // ``(idx)``, which is what real Tcl actively rejects
                // as ``can't read "arr": variable is array``.
                if (i < wlen and src[i] == '(') {
                    i += 1;
                    const ks = i;
                    while (i < wlen and src[i] != ')') i += 1;
                    const ke = i;
                    if (i < wlen) i += 1; // consume ')'
                    // Recursive call uses ``from_subst_cmd=false`` so
                    // any break/continue/return raised by ``[cmd]`` in
                    // the index span surfaces here as the rt.* flag.
                    // We then apply the outer subst command's
                    // exception-handling fold (parse-18.* / subst-8.9
                    // / 10.6 / 11.6).
                    const key_obj = subst_flagged_full(wptr + ks, ke - ks, do_vars, do_cmds, do_bs, false);
                    if (from_subst_cmd) {
                        const key_ir = result_mod.snapshot(key_obj);
                        switch (key_ir.code) {
                            .BREAK => {
                                result_mod.consume(.BREAK);
                                obj_mod.tcl_obj_release(name_obj);
                                if (key_obj != 0) obj_mod.tcl_obj_release(key_obj);
                                lit_start = i;
                                break;
                            },
                            .CONTINUE => {
                                result_mod.consume(.CONTINUE);
                                obj_mod.tcl_obj_release(name_obj);
                                if (key_obj != 0) obj_mod.tcl_obj_release(key_obj);
                                lit_start = i;
                                continue;
                            },
                            .RETURN => {
                                const rv = key_ir.value;
                                result_mod.consume(.RETURN);
                                obj_mod.tcl_obj_release(name_obj);
                                if (key_obj != 0) obj_mod.tcl_obj_release(key_obj);
                                if (rv != 0) {
                                    const sv = obj_ensure_string(rv);
                                    push_piece(pieces_buf, &n_pieces, &total_out, sv.ptr, sv.len);
                                    obj_mod.tcl_obj_retain(rv);
                                    write_i32(retained_objs + n_retained * 4, rv);
                                    n_retained += 1;
                                }
                                lit_start = i;
                                continue;
                            },
                            .OK, .ERROR => {},
                        }
                    }
                    // Route the read through ``var_resolve`` so it
                    // checks the proc-local frame for ``arr(key)``
                    // scalars FIRST (matching the write side in
                    // ``var_set``), then falls through to the global
                    // array directory.  Without this, ``$be(x,y)``
                    // inside an ``apply`` body always missed the
                    // local entry and returned empty (set-1.26).
                    const arr_s = obj_ensure_string(name_obj);
                    const key_s = obj_ensure_string(key_obj);
                    const full_len: u32 = arr_s.len + 2 + key_s.len;
                    const fbuf = alloc(full_len);
                    var elem: i32 = 0;
                    if (fbuf != 0) {
                        const fd: [*]u8 = @ptrFromInt(fbuf);
                        const ap: [*]const u8 = @ptrFromInt(arr_s.ptr);
                        for (0..arr_s.len) |k| fd[k] = ap[k];
                        fd[arr_s.len] = '(';
                        if (key_s.len > 0) {
                            const kp: [*]const u8 = @ptrFromInt(key_s.ptr);
                            for (0..key_s.len) |k| fd[arr_s.len + 1 + k] = kp[k];
                        }
                        fd[arr_s.len + 1 + key_s.len] = ')';
                        const full_obj = obj_new_string(@bitCast(fbuf), @bitCast(full_len));
                        elem = frames.var_resolve(full_obj);
                        obj_mod.tcl_obj_release(full_obj);
                    }
                    obj_mod.tcl_obj_release(name_obj);
                    if (key_obj != 0) obj_mod.tcl_obj_release(key_obj);
                    if (elem != 0) {
                        const sv = obj_ensure_string(elem);
                        push_piece(pieces_buf, &n_pieces, &total_out, sv.ptr, sv.len);
                        obj_mod.tcl_obj_retain(elem);
                        write_i32(retained_objs + n_retained * 4, elem);
                        n_retained += 1;
                    }
                } else {
                    const val = frames.var_resolve(name_obj);
                    obj_mod.tcl_obj_release(name_obj);
                    if (val != 0) {
                        const sv = obj_ensure_string(val);
                        push_piece(pieces_buf, &n_pieces, &total_out, sv.ptr, sv.len);
                        obj_mod.tcl_obj_retain(val);
                        write_i32(retained_objs + n_retained * 4, val);
                        n_retained += 1;
                    }
                }
            }
            lit_start = i;
        } else if (do_cmds and src[i] == '[') {
            flush_lit(pieces_buf, &n_pieces, &total_out, lit_start, &lit_run, wptr);
            i += 1;
            const cs = i;
            var depth: u32 = 1;
            var bk_at_word_start: bool = true;
            while (i < wlen and depth > 0) {
                const c = src[i];
                if (c == '\\' and i + 1 < wlen) {
                    // ``\<X>`` inside ``[...]`` consumes two bytes;
                    // depth is unaffected so a ``\[`` / ``\]`` inside
                    // a sub-script doesn't confuse the matcher.
                    i += 2;
                    continue;
                }
                if (c == ' ' or c == '\t' or c == '\n' or c == '\r' or c == ';') {
                    bk_at_word_start = true;
                    i += 1;
                    continue;
                }
                if (bk_at_word_start and c == '{') {
                    // Brace-quoted words inside ``[...]`` are literal —
                    // a ``[`` / ``]`` inside them must not affect
                    // bracket depth.  Only at word start; ``${foo}`` is
                    // a variable reference, not a brace word.
                    i += 1;
                    var bdepth: u32 = 1;
                    while (i < wlen and bdepth > 0) {
                        if (src[i] == '\\' and i + 1 < wlen) {
                            i += 2;
                            continue;
                        }
                        if (src[i] == '{') bdepth += 1
                        else if (src[i] == '}') bdepth -= 1;
                        i += 1;
                    }
                    bk_at_word_start = false;
                    continue;
                }
                if (bk_at_word_start and c == '"') {
                    // Quoted strings inside ``[...]`` — same reasoning:
                    // ``]`` inside is literal.
                    i += 1;
                    while (i < wlen and src[i] != '"') {
                        if (src[i] == '\\' and i + 1 < wlen) i += 2 else i += 1;
                    }
                    if (i < wlen) i += 1;
                    bk_at_word_start = false;
                    continue;
                }
                bk_at_word_start = false;
                if (c == '[') depth += 1 else if (c == ']') depth -= 1;
                if (depth > 0) i += 1 else i += 1;
            }
            // Unclosed ``[``: matches reference Tcl's ``TclSubstParse``
            // (tclParse.c) — the bracket body's complete commands must
            // run *before* the ``missing close-bracket`` error fires.
            // Walk the body command-by-command; the ``lastTerm`` cursor
            // tracks the end of the last successfully-parsed command,
            // and an incomplete trailing command (one whose parse runs
            // straight to end-of-input without a ``;`` / ``\n`` /
            // command-terminator) is excluded — its half-parsed shape
            // is what triggered the missing close-bracket.  Only the
            // ``subst`` command path applies this fix-up; word-level
            // substitution during normal script eval still surfaces the
            // bare error (the surrounding ``eval_script`` will report
            // it via the regular missing-close-bracket route).
            if (depth != 0) {
                if (from_subst_cmd and i > cs) {
                    const parse_mod = @import("tcl_parse.zig");
                    var pre_tok: [2 * parse_mod.MAX_WORDS]parse_mod.Token = undefined;
                    const body_ptr: [*]const u8 = @ptrFromInt(wptr + cs);
                    const body_len: u32 = i - cs;
                    var pre_pos: u32 = 0;
                    var last_term: u32 = 0;
                    while (pre_pos < body_len) {
                        const pcmd = parse_mod.ParseCommand(body_ptr, pre_pos, body_len, &pre_tok, pre_tok.len);
                        if (pcmd.next <= pre_pos) break;
                        // ``pcmd.next == body_len`` means the parse ran
                        // to end-of-input.  When the byte just before
                        // is a command terminator the previous command
                        // ended cleanly; otherwise this command itself
                        // is the incomplete one and we exclude it.
                        if (pcmd.next >= body_len) {
                            const last_byte = body_ptr[body_len - 1];
                            const term = (last_byte == ';' or last_byte == '\n' or last_byte == '\r');
                            if (term) last_term = body_len;
                            break;
                        }
                        last_term = pcmd.next;
                        pre_pos = pcmd.next;
                    }
                    if (last_term > 0) {
                        const interp = @import("../interp/tcl_interp.zig");
                        const result = interp.eval_script(wptr + cs, last_term);
                        const ir = result_mod.snapshot(result);
                        switch (ir.code) {
                            .BREAK => {
                                // ``break`` inside the bracket body
                                // suppresses the missing-close-bracket
                                // error and aborts the whole subst —
                                // matches subst-12.6 semantics.
                                result_mod.consume(.BREAK);
                                if (result != 0) obj_mod.tcl_obj_release(result);
                                lit_start = i;
                                var rj: u32 = 0;
                                while (rj < n_retained) : (rj += 1) {
                                    const r = read_i32(retained_objs + rj * 4);
                                    if (r != 0) obj_mod.tcl_obj_release(r);
                                }
                                arena.arena_free(retained_alloc);
                                arena.arena_free(pieces_alloc);
                                // Build the substituted prefix and
                                // return it cleanly.  Caller doesn't
                                // see an error at all on break.
                                return obj_new_string(@bitCast(wptr), @bitCast(lit_start));
                            },
                            .CONTINUE, .OK, .ERROR, .RETURN => {
                                // All other outcomes drop the body
                                // result and proceed to the deferred
                                // error below.  An ERROR from the
                                // body propagates regardless; consume
                                // CONTINUE/RETURN so the deferred
                                // ``missing close-bracket`` is what
                                // surfaces (per Bug 1036649 fix).
                                if (ir.code == .CONTINUE) result_mod.consume(.CONTINUE);
                                if (ir.code == .RETURN) result_mod.consume(.RETURN);
                                if (result != 0) obj_mod.tcl_obj_release(result);
                            },
                        }
                    }
                }
                var rj: u32 = 0;
                while (rj < n_retained) : (rj += 1) {
                    const r = read_i32(retained_objs + rj * 4);
                    if (r != 0) obj_mod.tcl_obj_release(r);
                }
                arena.arena_free(retained_alloc);
                arena.arena_free(pieces_alloc);
                return raise_subst_error("missing close-bracket");
            }
            const ce = i - 1;
            // Tcl 9 ``subst`` pre-parses the bracket body to bytecode.
            // When the parse fails partway (e.g. ``[continue; expr;
            // set {} {}{}]`` has an "extra characters after
            // close-brace" error in the third command), Tcl saves the
            // error, compiles the prefix that DID parse cleanly, and
            // raises the saved error only after running that prefix.
            // ``[break]`` exits cleanly without firing the error;
            // continue/return/normal-completion all end up firing it.
            //
            // Mirror that behaviour: pre-walk the bracket body
            // command-by-command, find the prefix that parses cleanly,
            // run that prefix, then (for ``subst`` cmd context) fire
            // the saved error unless ``break_flag`` was raised
            // (parse-18.19 / 18.20 / 18.21).
            var eval_len: u32 = ce - cs;
            var deferred_err: bool = false;
            var deferred_err_quote: bool = false;
            if (from_subst_cmd and ce > cs) {
                const parse_mod = @import("tcl_parse.zig");
                var pre_pos: u32 = 0;
                var pre_tok: [2 * parse_mod.MAX_WORDS]parse_mod.Token = undefined;
                const body_ptr: [*]const u8 = @ptrFromInt(wptr + cs);
                const body_len: u32 = ce - cs;
                while (pre_pos < body_len) {
                    const pcmd = parse_mod.ParseCommand(body_ptr, pre_pos, body_len, &pre_tok, pre_tok.len);
                    if (pcmd.extra_chars_after_close) {
                        // The failing command starts at pre_pos.
                        // Eval the prefix [0..pre_pos) and remember to
                        // raise the deferred error if we don't break.
                        eval_len = pre_pos;
                        deferred_err = true;
                        deferred_err_quote = pcmd.extra_chars_after_quote;
                        break;
                    }
                    if (pcmd.next <= pre_pos) break;
                    pre_pos = pcmd.next;
                }
            }
            const interp = @import("../interp/tcl_interp.zig");
            const result = interp.eval_script(wptr + cs, eval_len);
            if (from_subst_cmd) {
                const ir = result_mod.snapshot(result);
                switch (ir.code) {
                    .BREAK => {
                        // Break cleanly aborts the entire subst — even a
                        // pending deferred parse error is dropped (Tcl 9
                        // semantics, parse-18.18 vs parse-18.19).
                        result_mod.consume(.BREAK);
                        lit_start = i;
                        break;
                    },
                    .CONTINUE => {
                        result_mod.consume(.CONTINUE);
                        if (deferred_err) {
                            var rj: u32 = 0;
                            while (rj < n_retained) : (rj += 1) {
                                const r = read_i32(retained_objs + rj * 4);
                                if (r != 0) obj_mod.tcl_obj_release(r);
                            }
                            arena.arena_free(retained_alloc);
                            arena.arena_free(pieces_alloc);
                            return raise_subst_error(if (deferred_err_quote) "extra characters after close-quote" else "extra characters after close-brace");
                        }
                        lit_start = i;
                        continue;
                    },
                    .RETURN => {
                        const rv = ir.value;
                        result_mod.consume(.RETURN);
                        if (deferred_err) {
                            if (rv != 0) obj_mod.tcl_obj_release(rv);
                            var rj: u32 = 0;
                            while (rj < n_retained) : (rj += 1) {
                                const r = read_i32(retained_objs + rj * 4);
                                if (r != 0) obj_mod.tcl_obj_release(r);
                            }
                            arena.arena_free(retained_alloc);
                            arena.arena_free(pieces_alloc);
                            return raise_subst_error(if (deferred_err_quote) "extra characters after close-quote" else "extra characters after close-brace");
                        }
                        if (rv != 0) {
                            const sv = obj_ensure_string(rv);
                            push_piece(pieces_buf, &n_pieces, &total_out, sv.ptr, sv.len);
                            obj_mod.tcl_obj_retain(rv);
                            write_i32(retained_objs + n_retained * 4, rv);
                            n_retained += 1;
                        }
                        lit_start = i;
                        continue;
                    },
                    .OK, .ERROR => {},
                }
            }
            // Non-subst-cmd path: any break/continue/return flags
            // remain set on rt.* and propagate to the enclosing
            // ``eval_script`` loop, which will handle them after this
            // word's substitution completes.  We deliberately don't
            // bail early here — a partial substitution result is still
            // safe to return; the caller's flag check supersedes it.
            if (result != 0) {
                const sv = obj_ensure_string(result);
                push_piece(pieces_buf, &n_pieces, &total_out, sv.ptr, sv.len);
                obj_mod.tcl_obj_retain(result);
                write_i32(retained_objs + n_retained * 4, result);
                n_retained += 1;
            }
            // Normal-completion deferred-error fire (parse-18.19 etc):
            // when the bracket body parsed cleanly up to N commands but
            // command N+1 had a parse error, raise that error AFTER
            // running the prefix.  Only the ``subst`` command path
            // owns this fold (subst_word leaves it to the caller).
            if (from_subst_cmd and deferred_err) {
                var rj: u32 = 0;
                while (rj < n_retained) : (rj += 1) {
                    const r = read_i32(retained_objs + rj * 4);
                    if (r != 0) obj_mod.tcl_obj_release(r);
                }
                arena.arena_free(retained_alloc);
                arena.arena_free(pieces_alloc);
                return raise_subst_error(if (deferred_err_quote) "extra characters after close-quote" else "extra characters after close-brace");
            }
            lit_start = i;
        } else if (do_bs and src[i] == '\\' and i + 1 < wlen) {
            flush_lit(pieces_buf, &n_pieces, &total_out, lit_start, &lit_run, wptr);
            // S6.3: previously this 4-byte escape buffer was
            // ``alloc()``-ed and never freed (each call leaked a
            // size-class slab).  Routing through the arena both
            // fixes the leak and avoids the libc round-trip.
            const esc_alloc = arena.arena_alloc_or_libc(4);
            const esc_ptr = esc_alloc.addr;
            if (esc_ptr == 0) { lit_run += wlen - i; break; }
            const r = obj_mod.consume_bs_escape(src, i + 1, wlen, @ptrFromInt(esc_ptr));
            i = r.next_si;
            push_piece(pieces_buf, &n_pieces, &total_out, esc_ptr, r.written);
            lit_start = i;
        } else {
            lit_run += 1;
            i += 1;
        }
    }
    flush_lit(pieces_buf, &n_pieces, &total_out, lit_start, &lit_run, wptr);

    const out_cap: u32 = total_out + 1;
    const buf = alloc(out_cap);
    if (buf == 0) {
        // Final concat buffer OOM.  Release any sources we've
        // retained so we don't compound an OOM with a refcount
        // leak.  Scratch cleanup happens via the deferred
        // ``arena_restore`` at the outer scope; ``arena_free``
        // routes any libc-fallback allocations correctly.
        var rj: u32 = 0;
        while (rj < n_retained) : (rj += 1) {
            const r = read_i32(retained_objs + rj * 4);
            if (r != 0) obj_mod.tcl_obj_release(r);
        }
        arena.arena_free(retained_alloc);
        arena.arena_free(pieces_alloc);
        return obj_new_string(0, 0);
    }
    var out: u32 = 0;
    var pi: u32 = 0;
    while (pi < n_pieces) : (pi += 1) {
        const slot = pieces_buf + pi * 8;
        const p: u32 = @bitCast(read_i32(slot));
        const l: u32 = @bitCast(read_i32(slot + 4));
        if (l > 0) {
            memcpy(buf + out, p, l);
            out += l;
        }
    }
    // MM-B.6: now that the bytes are copied, release the retained
    // sources.  S6.3: scratch buffers (``pieces_buf`` /
    // ``retained_objs`` / per-escape ``esc_ptr``) are reclaimed by
    // the deferred ``arena_restore`` — ``arena_free`` only does
    // work for libc-fallback allocations that overflowed the
    // arena, so it's safe (and necessary) to call regardless.
    var ri: u32 = 0;
    while (ri < n_retained) : (ri += 1) {
        const r = read_i32(retained_objs + ri * 4);
        if (r != 0) obj_mod.tcl_obj_release(r);
    }
    arena.arena_free(retained_alloc);
    arena.arena_free(pieces_alloc);
    // NOTE: deliberately NOT claiming ownership of ``buf`` via
    // OBJ_STR_CAP.  Callers (e.g. ``array set arr [subst …]``)
    // borrow ``(ptr, len)`` slices out of the returned TclObj's
    // bytes and stash them elsewhere without retaining the source.
    // Setting OBJ_STR_CAP would make a later release of the source
    // free the buffer mid-flight and expose those borrowers to a
    // use-after-free.  Until the array/list set helpers either
    // retain the source or copy bytes, subst_flagged keeps the
    // bytes alive (treated as a literal buffer) — same contract
    // the implementation had before this OOM-hardening pass.
    return obj_new_string(@bitCast(buf), @bitCast(out));
}
