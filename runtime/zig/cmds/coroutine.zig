// ``coroutine`` / ``yield`` / ``yieldto`` — segment-based v1.
//
// See ``runtime/zig/sched/tcl_coro.zig`` for the model and its
// limitations.  This file is the command-registration thin wrapper.

const reg = @import("../dispatch/tcl_cmd_registry.zig");
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
    return obj.obj_new_string(@bitCast(buf), @bitCast(off));
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

fn eval_coroutine(words: []const i32) i32 {
    if (words.len < 3) {
        stubs.raise("wrong # args: should be \"coroutine name command ?arg ...?\"");
        return 0;
    }
    // Build the body as a single list-quoted script so the
    // dispatcher invokes ``prefix args...`` exactly once.  Under
    // the asyncify driver the resulting script runs end-to-end
    // (yields suspend at any depth), so no segment-aware shaping
    // is needed.  Under the v1 segment driver this body is split
    // at top-level command boundaries — the caller is responsible
    // for shaping the prefix so the splits land where intended.
    var body: i32 = 0;
    // v1 segment-based driver special case: when the prefix is
    // ``apply LAMBDA`` extract the lambda body so segment splits
    // at top-level yields actually fire.  Asyncify-enabled builds
    // skip this — the full lambda runs as one script.
    if (!tcl_async.ENABLED and words.len == 4) {
        const w2 = obj.obj_ensure_string(words[2]);
        const w2s: []const u8 = if (w2.ptr == 0) "" else
            @as([*]const u8, @ptrFromInt(w2.ptr))[0..w2.len];
        if (w2s.len == 5 and w2s[0] == 'a' and w2s[1] == 'p' and
            w2s[2] == 'p' and w2s[3] == 'l' and w2s[4] == 'y')
        {
            const lam = obj.obj_ensure_string(words[3]);
            const n_parts = obj.list_count_elements(lam.ptr, lam.len);
            if (n_parts >= 2) {
                const body_elem = obj.list_element_at(lam.ptr, lam.len, 1);
                body = if (body_elem.braced)
                    obj.obj_new_string_copy(lam.ptr + body_elem.start, body_elem.len)
                else
                    obj.obj_new_string(@bitCast(lam.ptr + body_elem.start), @bitCast(body_elem.len));
            }
        }
    }
    if (body == 0) {
        body = build_invocation_script(words[2..]);
    }
    const name = obj.obj_ensure_string(words[1]);
    const c = coro_mod.create(name.ptr, name.len, body) orelse {
        stubs.raise("coroutine table full or name already in use");
        // ``create`` retains body on success; on failure the +1
        // from build_invocation_script / obj_new_string_copy is the
        // caller's, so release it here.
        if (body != 0) obj.tcl_obj_release(body);
        return 0;
    };
    // ``create`` retains body — drop our +1 from the build step
    // above so the coroutine slot is the sole owner.  Without this
    // the body's refcount stays artificially high for the
    // coroutine's lifetime (Copilot review on PR #284).
    obj.tcl_obj_release(body);

    // Register the coroutine as a CMD_COROUTINE command in the
    // current namespace.
    const r = tcl_ns.ns_resolve_qualified_creating(
        tcl_ns.ns_current(), name.ptr, name.len,
    );
    if (r.target_ns == 0 or r.simple_len == 0) {
        stubs.raise("invalid coroutine name");
        return 0;
    }
    // Build the FQN once — used both as the Command's name slot
    // (for introspection consistency with ``proc_register``) and
    // as the ``[c]`` return value.  Real Tcl returns the FQN of
    // the new coroutine command, not just the simple name.
    const fqn = tcl_ns.ns_build_fqn(r.target_ns, r.simple_ptr, r.simple_len);
    const coro_addr: u32 = @intCast(@intFromPtr(c));
    const cmd = build_coro_command(coro_addr, fqn.ptr, fqn.len);
    _ = tcl_ns.ns_cmd_put(r.target_ns, r.simple_ptr, r.simple_len, cmd);
    procs.proc_count_bump();
    coro_mod.record_registration(c, r.target_ns, r.simple_ptr, r.simple_len);
    // First-time invocation: real Tcl calls the body once before
    // returning the coroutine name.  Our segment-based model does
    // this implicitly the first time the coro is called by the
    // user — for simplicity v1 returns the FQN now and lets the
    // user invoke ``[name]`` to start the body.  This is a v1
    // semantic deviation; documented as such.
    return obj.obj_new_string(@bitCast(fqn.ptr), @bitCast(fqn.len));
}

fn eval_yield(words: []const i32) i32 {
    if (words.len > 2) {
        stubs.raise("wrong # args: should be \"yield ?value?\"");
        return 0;
    }
    if (!coro_mod.current_in_coroutine()) {
        stubs.raise("yield can only be called in a coroutine");
        return 0;
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
    return 0;
}

fn eval_yieldto(words: []const i32) i32 {
    if (words.len < 2) {
        stubs.raise("wrong # args: should be \"yieldto command ?arg ...?\"");
        return 0;
    }
    if (!coro_mod.current_in_coroutine()) {
        stubs.raise("yieldto can only be called in a coroutine");
        return 0;
    }
    // Invoke the target command as ``[command args...]`` and yield
    // its result.  ``concat_words`` returns ``ws[0]`` borrowed when
    // there is a single word and a fresh +1 TclObj otherwise; track
    // ownership so we release the temporary in the multi-word case.
    const interp = @import("../interp/tcl_interp.zig");
    const multi_word = words[1..].len > 1;
    const concat = interp.concat_words(words[1..]);
    const cs = obj.obj_ensure_string(concat);
    const result = interp.eval_script(cs.ptr, cs.len);
    if (multi_word) obj.tcl_obj_release(concat);
    _ = coro_mod.signal_yield(result);
    return 0;
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "coroutine", .arity_min = 2, .arity_max = null, .handler = &eval_coroutine },
    .{ .name = "yield", .arity_min = 0, .arity_max = 1, .handler = &eval_yield },
    .{ .name = "yieldto", .arity_min = 1, .arity_max = null, .handler = &eval_yieldto },
};
