// ``coroutine`` / ``yield`` / ``yieldto`` — segment-based v1.
//
// See ``runtime/zig/sched/tcl_coro.zig`` for the model and its
// limitations.  This file is the command-registration thin wrapper.

const reg = @import("../dispatch/tcl_cmd_registry.zig");
const result_mod = @import("../interp/tcl_result.zig");
const catch_mod = @import("../interp/tcl_catch.zig");
const obj = @import("../valtypes/tcl_obj.zig");
const stubs = @import("../stubs/tcl_stubs.zig");
const coro_mod = @import("../sched/tcl_coro.zig");
const tcl_async = @import("../sched/tcl_asyncify.zig");
const procs = @import("../interp/tcl_procs.zig");
const tcl_ns = @import("../interp/tcl_ns.zig");

fn write_i32(addr: u32, value: i32) void {
    obj.write_i32(addr, value);
}

/// Build ``cmd ?arg arg…?`` as a single script string with each
/// element rendered through the list-element quoter so braces
/// preserve word boundaries.  Mirrors :func:`build_invocation_list`
/// in ``tcl_interp.zig`` but exposed here without making that
/// helper public.
fn build_invocation_script(ws: []const i32) i32 {
    if (ws.len == 0) return obj.obj_new_string(0, 0);
    var total: u32 = 0;
    var i: usize = 0;
    while (i < ws.len) : (i += 1) {
        const s = obj.obj_ensure_string(ws[i]);
        // Worst case: 2x the source bytes plus brace wrap.
        total += s.len * 2 + 4;
        if (i > 0) total += 1; // separator space
    }
    const buf = obj.alloc(total);
    if (buf == 0) return obj.obj_new_string(0, 0);
    var off: u32 = 0;
    i = 0;
    while (i < ws.len) : (i += 1) {
        if (i > 0) {
            const dst: [*]u8 = @ptrFromInt(buf + off);
            dst[0] = ' ';
            off += 1;
        }
        const s = obj.obj_ensure_string(ws[i]);
        if (i == 0) {
            off = obj.list_elem_quote(buf, off, s.ptr, s.len);
        } else {
            off = obj.list_elem_quote_nth(buf, off, s.ptr, s.len);
        }
    }
    return obj.obj_new_string_take(buf, off, total);
}

/// Build the dispatch ``Command`` struct.  ``fqn_ptr`` / ``fqn_len``
/// is the heap-copied fully-qualified name (``::ns::name``) the
/// Command's name slot stores — matches the shape ``proc_register``
/// uses so introspection (``info procs``, ``namespace which``)
/// reports the same FQN for coroutines as for procs.
fn build_coro_command(coro_ptr: u32, fqn_ptr: u32, fqn_len: u32) u32 {
    const cmd = obj.alloc(procs.COMMAND_SIZE);
    const slice: [*]u8 = @ptrFromInt(cmd);
    @memset(slice[0..procs.COMMAND_SIZE], 0);
    write_i32(cmd, @bitCast(fqn_ptr));
    write_i32(cmd + 4, @bitCast(fqn_len));
    // Flags + params_obj = *Coro.
    write_i32(cmd + procs.OFF_FLAGS, @bitCast(procs.CMD_COROUTINE));
    write_i32(cmd + procs.OFF_PARAMS_OBJ, @bitCast(coro_ptr));
    return cmd;
}

fn eval_coroutine(words: []const i32) result_mod.InterpResult {
    if (words.len < 3) {
        stubs.raise("wrong # args: should be \"coroutine name command ?arg ...?\"");
        return result_mod.from_globals(0);
    }
    // Build the body as a single list-quoted script so the
    // dispatcher invokes ``prefix args...`` exactly once.  Under
    // the asyncify driver the resulting script runs end-to-end
    // (yields suspend at any depth), so no segment-aware shaping
    // is needed.  Under the v1 segment driver this body is split
    // at top-level command boundaries — the caller is responsible
    // for shaping the prefix so the splits land where intended.
    var body: i32 = 0;
    // Track whether the coroutine prefix is a frame-creating
    // wrapper (``apply`` / ``proc``) that absorbs a body-level
    // ``return`` rather than letting it propagate out.  Used by
    // the eager-invocation re-arm path below: for absorbing
    // wrappers we must NOT reinstate the body's RETURN flag,
    // otherwise the surrounding ``puts [coroutine c apply {{}
    // {return done}}]`` site would observe a phantom RETURN
    // (regression captured by
    // ``test_coroutine_command_removed_after_terminal_return``).
    var absorbs_return = false;
    // v1 segment-based driver special case: when the prefix is
    // ``apply LAMBDA`` or ``eval BODY`` extract the inner script
    // body so segment splits at top-level yields actually fire.
    // Asyncify-enabled builds skip this — the full prefix runs as
    // one script and asyncify suspends mid-stack regardless of
    // segment boundaries.
    if (!tcl_async.ENABLED and words.len == 4) {
        const w2 = obj.obj_ensure_string(words[2]);
        const w2s: []const u8 = if (w2.ptr == 0) "" else @as([*]const u8, @ptrFromInt(w2.ptr))[0..w2.len];
        if (w2s.len == 5 and w2s[0] == 'a' and w2s[1] == 'p' and
            w2s[2] == 'p' and w2s[3] == 'l' and w2s[4] == 'y')
        {
            absorbs_return = true;
            const lam = obj.obj_ensure_string(words[3]);
            const n_parts = obj.list_count_elements(lam.ptr, lam.len);
            if (n_parts >= 2) {
                const body_elem = obj.list_element_at(lam.ptr, lam.len, 1);
                // Always copy the bytes — the unbraced branch used to
                // ``obj_new_string`` (borrow) wrapping ``lam.ptr +
                // body_elem.start``, but ``lam.ptr`` is borrowed from
                // ``words[3]``'s buffer.  ``coro_mod.create`` retains
                // the body TclObj header, but its ``str_ptr`` still
                // pointed into ``words[3]``'s buffer.  After dispatch
                // releases ``words[3]`` the buffer is freed and the
                // coroutine's body string-rep dangles.
                body = obj.obj_new_string_copy(lam.ptr + body_elem.start, body_elem.len);
            }
        } else if (w2s.len == 4 and w2s[0] == 'e' and w2s[1] == 'v' and
            w2s[2] == 'a' and w2s[3] == 'l')
        {
            // ``coroutine NAME eval {script}`` — strip the outer
            // ``eval`` wrapper and use the script directly so
            // ``split_segments`` sees each top-level statement
            // (each ``yield`` / ``yieldto``) as its own segment.
            // Without this the body was the whole
            // ``eval {…}`` literal and segment splitting found a
            // single segment, which made the second ``[demo]``
            // resume hit ``next_segment >= n_segments`` and
            // immediately mark the coroutine DEAD —
            // coroutine.test 11.1 (and the multi-yield 1.13 /
            // 1.14 patterns once they reach this path).
            const inner = obj.obj_ensure_string(words[3]);
            body = obj.obj_new_string_copy(inner.ptr, inner.len);
        }
    }
    if (body == 0) {
        body = build_invocation_script(words[2..]);
    }
    const name = obj.obj_ensure_string(words[1]);
    // Resolve the name BEFORE creating the coroutine record so the
    // table dedup key is the resolved FQN, not the raw user name.
    // Without this, ``coroutine b`` (inside ``namespace eval ::a``)
    // and ``coroutine ::a::b`` would create two distinct coroutine
    // records sharing the same ``::a::b`` ``cmd_table`` entry — the
    // second registration would overwrite the first and leak the
    // displaced coroutine + its body (Copilot review on PR #284).
    const r = tcl_ns.ns_resolve_qualified_creating(
        tcl_ns.ns_current(),
        name.ptr,
        name.len,
    );
    if (r.target_ns == 0 or r.simple_len == 0) {
        stubs.raise("invalid coroutine name");
        if (body != 0) obj.tcl_obj_release(body);
        return result_mod.from_globals(0);
    }
    const fqn = tcl_ns.ns_build_fqn(r.target_ns, r.simple_ptr, r.simple_len);
    const c = coro_mod.create(fqn.ptr, fqn.len, body) orelse {
        stubs.raise("coroutine table full or name already in use");
        // ``create`` retains body on success; on failure the +1
        // from build_invocation_script / obj_new_string_copy is the
        // caller's, so release it here.
        if (body != 0) obj.tcl_obj_release(body);
        return result_mod.from_globals(0);
    };
    // ``create`` retains body — drop our +1 from the build step
    // above so the coroutine slot is the sole owner.  Without this
    // the body's refcount stays artificially high for the
    // coroutine's lifetime (Copilot review on PR #284).
    obj.tcl_obj_release(body);
    const coro_addr: u32 = @intCast(@intFromPtr(c));
    const cmd = build_coro_command(coro_addr, fqn.ptr, fqn.len);
    _ = tcl_ns.ns_cmd_put(r.target_ns, r.simple_ptr, r.simple_len, cmd);
    procs.proc_count_bump();
    coro_mod.record_registration(c, r.target_ns, r.simple_ptr, r.simple_len);
    // Tcl 9 ``coroutine`` semantics (``Tcl_CoroutineObjCmd``):
    // the body runs once before the command returns.  If the body
    // yields, the first yield value becomes the result and the
    // coroutine command remains for further ``[name]`` resumes; if
    // the body completes without yielding, the coroutine command
    // is auto-removed (``cleanup_terminated`` runs inside
    // ``resume_one`` when ``c.state == .DEAD``) and the body's
    // terminal result is returned, with any non-OK return code
    // (ERROR / RETURN / BREAK / CONTINUE / custom) propagated to
    // the caller.
    //
    // The previous v1 model returned the FQN immediately and
    // deferred the body to the first ``[name]`` call.  That left
    // ``catch {coroutine demo return -level 0 -code N}`` (test
    // 7.5) leaking demo across loop iterations and made nested
    // ``coroutine X coroutine Y …`` (test 3.7) report the wrong
    // ``info frame`` content because the inner body never ran.
    const result = coro_mod.resume_one(c);
    // ``resume_segments`` consumes RETURN inside its hot loop so
    // a script-level ``[c]`` resume that terminates doesn't leak
    // the flag into the next top-level command's catch (the
    // regression captured by
    // ``test_coroutine_command_removed_after_terminal_return``).
    // For the eager-invocation path we want the opposite: the
    // body's terminal code IS the result of this ``coroutine``
    // command, and a wrapping catch must see it.  Read the
    // terminal snapshot ``resume_one`` stashed before cleanup
    // and re-arm RETURN when applicable.  ERROR is never
    // consumed by ``resume_segments`` so it carries through
    // verbatim.  BREAK / CONTINUE pass through their flag-based
    // mechanisms — neither is consumed in the segment loop.
    //
    // EXCEPTION: when the coroutine prefix is a frame-creating
    // wrapper like ``apply`` (or, in the asyncify path, ``proc``),
    // reference Tcl absorbs the body-level RETURN at the wrapper
    // boundary instead of propagating it.  Our v1 segment driver
    // strips the wrapper so the body runs without that absorbing
    // frame; ``absorbs_return`` opts the eager-invocation site
    // out of the re-arm so the surrounding script sees the
    // absorbed (OK) outcome rather than a phantom RETURN.
    const term = coro_mod.take_last_terminal();
    const RETURN_CODE: u8 = 2;
    if (term.code == RETURN_CODE and !absorbs_return) {
        result_mod.set_return(result, 0);
        catch_mod.state.return_code = term.return_code;
    }
    return result_mod.from_globals(result);
}

fn eval_yield(words: []const i32) result_mod.InterpResult {
    if (words.len > 2) {
        stubs.raise("wrong # args: should be \"yield ?value?\"");
        return result_mod.from_globals(0);
    }
    if (!coro_mod.current_in_coroutine()) {
        stubs.raise("yield can only be called in a coroutine");
        return result_mod.from_globals(0);
    }
    // Pass ``words[1]`` (a parser-owned TclObj — no extra retain)
    // when present, else the sentinel ``0`` so ``signal_yield``
    // doesn't allocate a new empty TclObj that would never be
    // released.  The yielded value is carried back to the [c]
    // caller via ``yield_value``; eval_yield's own return is
    // unused (yield always unwinds to ``tcl_coro_drive``), so
    // returning ``0`` avoids a phantom +1 ownership on the
    // caller.  Codex / Copilot review on PR #284.
    const value: i32 = if (words.len == 2) words[1] else 0;
    _ = coro_mod.signal_yield(value);
    coro_mod.record_yield_kind(1);
    return result_mod.from_globals(0);
}

fn eval_yieldto(words: []const i32) result_mod.InterpResult {
    if (words.len < 2) {
        stubs.raise("wrong # args: should be \"yieldto command ?arg ...?\"");
        return result_mod.from_globals(0);
    }
    if (!coro_mod.current_in_coroutine()) {
        stubs.raise("yieldto can only be called in a coroutine");
        return result_mod.from_globals(0);
    }
    // Invoke the target command as ``[command args...]`` and yield
    // its result.  ``concat_words`` returns ``ws[0]`` borrowed when
    // there is a single word and a fresh +1 TclObj otherwise; track
    // ownership so we release the temporary in the multi-word case.
    const interp = @import("../interp/tcl_interp.zig");
    // Dispatch the command directly via ``eval_call`` so each
    // word retains its identity — the previous concat-and-eval
    // path joined the args with spaces and re-parsed them, which
    // turned ``yieldto string cat "PHASE 2"`` into a four-word
    // ``string cat PHASE 2`` invocation (the embedded space
    // split the third arg) and ``string cat`` produced
    // ``PHASE2`` instead of ``PHASE 2`` (coroutine.test 11.1).
    // ``eval_call`` is the same entry point ``apply`` uses for
    // its lambda-body dispatch, so word boundaries are preserved.
    const result = interp.eval_call(words[1..]);
    // Materialise an owned copy of the result string in case
    // it borrows bytes from a parser-side buffer that ``signal_yield``
    // can't keep alive across the suspend (e.g. an inline
    // string-cat-of-single-arg returns the input verbatim,
    // whose backing buffer lives on the dispatcher stack).
    const owned: i32 = if (result != 0) blk: {
        const rs = obj.obj_ensure_string(result);
        break :blk obj.obj_new_string_copy(rs.ptr, rs.len);
    } else 0;
    _ = coro_mod.signal_yield(owned);
    if (owned != 0) obj.tcl_obj_release(owned);
    coro_mod.record_yield_kind(2);
    return result_mod.from_globals(0);
}

/// ``::tcl::unsupported::corotype CORONAME`` returns the
/// suspension state of the named coroutine: ``yield`` if the last
/// suspension was a plain ``yield``, ``yieldto`` if the last
/// suspension was via ``yieldto``, or ``active`` when invoked from
/// within the coroutine itself (``[info coroutine]`` returns its
/// own name and the coro is currently running).  Mirrors Tcl 9's
/// ``Tcl_CorotypeObjCmd`` (``generic/tclBasic.c``); coroutine.test
/// 11.1 round-trips all three states.
fn eval_corotype(words: []const i32) result_mod.InterpResult {
    if (words.len != 2) {
        stubs.raise("wrong # args: should be \"::tcl::unsupported::corotype coroName\"");
        return result_mod.from_globals(0);
    }
    const name = obj.obj_ensure_string(words[1]);
    if (name.len == 0) {
        stubs.raise("can only get coroutine type of a coroutine");
        return result_mod.from_globals(0);
    }
    // Resolve the user-supplied name to a fully-qualified FQN
    // (``::demo``) so the lookup matches the registration done
    // inside ``eval_coroutine`` (which always stores the resolved
    // FQN).  ``ns_resolve_qualified`` returns the simple-name span
    // and target namespace; we re-build the FQN here.
    const r = tcl_ns.ns_resolve_qualified(tcl_ns.ns_current(), name.ptr, name.len);
    if (r.target_ns == 0 or r.simple_len == 0) {
        stubs.raise("can only get coroutine type of a coroutine");
        return result_mod.from_globals(0);
    }
    const fqn = tcl_ns.ns_build_fqn(r.target_ns, r.simple_ptr, r.simple_len);
    const c = coro_mod.lookup(fqn.ptr, fqn.len);
    if (c == null) {
        stubs.raise("can only get coroutine type of a coroutine");
        return result_mod.from_globals(0);
    }
    const lit: []const u8 = switch (c.?.state) {
        .RUNNING => "active",
        .SUSPENDED => switch (c.?.last_yield_kind) {
            2 => "yieldto",
            else => "yield",
        },
        .DEAD => "dead",
        else => "active",
    };
    return result_mod.from_globals(obj.obj_new_string_copy(@intFromPtr(lit.ptr), @intCast(lit.len)));
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "coroutine", .arity_min = 2, .arity_max = null, .handler = &eval_coroutine },
    .{ .name = "yield", .arity_min = 0, .arity_max = 1, .handler = &eval_yield },
    .{ .name = "yieldto", .arity_min = 1, .arity_max = null, .handler = &eval_yieldto },
    .{ .name = "::tcl::unsupported::corotype", .arity_min = 1, .arity_max = 1, .handler = &eval_corotype },
};
