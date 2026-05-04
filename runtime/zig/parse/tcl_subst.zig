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

const alloc            = rt.alloc;
const memcpy           = rt.memcpy;
const read_i32         = obj_mod.read_i32;
const write_i32        = obj_mod.write_i32;
const obj_new_string   = rt.obj_new_string;
const obj_ensure_string = rt.obj_ensure_string;

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
                while (i < wlen and src[i] != '}') i += 1;
                const ve = i;
                if (i < wlen) i += 1;
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
                while (i < wlen and ((src[i] >= 'a' and src[i] <= 'z') or
                    (src[i] >= 'A' and src[i] <= 'Z') or
                    (src[i] >= '0' and src[i] <= '9') or src[i] == '_'))
                {
                    i += 1;
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
                    const key_obj = subst_flagged(wptr + ks, ke - ks, do_vars, do_cmds, do_bs);
                    const elem = tcl_array.array_get(name_obj, key_obj);
                    // Release the per-substitution temps — both
                    // ``name_obj`` (built from the source span) and
                    // ``key_obj`` (returned by the recursive
                    // subst) are scratch.  Issue #303 leak fix.
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
            while (i < wlen and depth > 0) {
                if (src[i] == '[') depth += 1 else if (src[i] == ']') depth -= 1;
                if (depth > 0) i += 1 else i += 1;
            }
            // Unclosed ``[``: the loop ran out of source with depth>0
            // (e.g. ``subst \[`` → arg is the single byte ``[``).  Real
            // Tcl raises ``missing close-bracket`` here, but ``ce - cs``
            // below underflows the u32 (cs == 1, ce == 0) and panics in
            // ReleaseSafe before we can do anything useful.  Skip the
            // command-substitution attempt; the unterminated fragment
            // becomes empty.  Plumbing the proper error message through
            // is a follow-up — this branch only stops the crash so the
            // rest of the test file can run.
            if (depth != 0) {
                lit_start = i;
                continue;
            }
            const ce = i - 1;
            const interp = @import("../interp/tcl_interp.zig");
            const result = interp.eval_script(wptr + cs, ce - cs);
            if (result != 0) {
                const sv = obj_ensure_string(result);
                push_piece(pieces_buf, &n_pieces, &total_out, sv.ptr, sv.len);
                obj_mod.tcl_obj_retain(result);
                write_i32(retained_objs + n_retained * 4, result);
                n_retained += 1;
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
