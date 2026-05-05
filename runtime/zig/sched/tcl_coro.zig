// Coroutine support — Stage 2 (asyncify-backed) with v1 fallback.
//
// Two drivers live behind a single :func:`resume_one` entry point:
//
//   * **v1 (segment-based)** — default build.  Body is split at
//     top-level command boundaries (semicolons / newlines outside
//     braces) and each ``[c]`` evaluates the next segment.  Yield
//     only works as a top-level command; nested yields fail.  This
//     is what runs in the standard ``zig build`` artefact.
//
//   * **Stage 2 (asyncify)** — opt-in via ``-Dasyncify=true``.  The
//     runtime is post-processed with ``wasm-opt --asyncify`` so calls
//     to the ``asyncify_*`` intrinsics declared in
//     :file:`tcl_asyncify.zig` perform full call-stack save/restore.
//     :func:`tcl_coro_drive` is in the asyncify removelist
//     (``--pass-arg=asyncify-removelist@sched.tcl_coro.tcl_coro_drive``)
//     so the unwind triggered by yield stops at the driver instead
//     of propagating to the host.
//
// **Status of the asyncify path (Stage 2.5 — issue #282):**
//
//   * Single-yield works correctly — yield from arbitrary call
//     depth (apply / proc / nested loops) unwinds cleanly to the
//     driver and returns the yielded value.
//   * Multi-yield rewind works: ``signal_yield`` calls
//     :func:`tcl_async.coro_yield_unwind` (a host-trampolined
//     import marked ``--pass-arg=asyncify-imports@env.coro_yield_unwind``).
//     The trampoline routes on the asyncify state — ``NORMAL``
//     triggers ``asyncify_start_unwind`` (begin unwind); ``REWINDING``
//     triggers ``asyncify_stop_rewind`` (transition to ``NORMAL`` so
//     the body resumes past the original ``yield`` to the next).
//     This is the standard Emscripten asyncify pattern for
//     resumable coroutines — calling ``asyncify_start_unwind``
//     directly here would loop on resume because the rewind dispatch
//     re-fires the saved call site.
//   * Body completion past the last yield (``return X`` or natural
//     end) currently traps under asyncify when reached via a resume.
//     Tracked as Stage 2.6; the v1 segment driver is selected by
//     default so this doesn't affect production use.
//
// Both drivers share the public surface (:func:`create`,
// :func:`lookup`, :func:`resume_one`, :func:`signal_yield`,
// :func:`current_in_coroutine`) so the dispatch site in
// :func:`eval_proc_call_bucket` doesn't care which model is active.

const std = @import("std");
const obj = @import("../valtypes/tcl_obj.zig");
const tcl_obj_retain = obj.tcl_obj_retain;
const tcl_obj_release = obj.tcl_obj_release;
const obj_new_string = obj.obj_new_string;
const obj_ensure_string = obj.obj_ensure_string;

const tcl_catch = @import("../interp/tcl_catch.zig");
const tcl_frames = @import("../interp/tcl_frames.zig");
const result_mod = @import("../interp/tcl_result.zig");
const tcl_async = @import("tcl_asyncify.zig");

// The yield signal flag lives on ``tcl_catch.state.yield_flag`` so
// ``has_signal()`` in the interpreter sees it without a module-
// circular import.  Reads/writes go through tcl_catch directly.

const MAX_COROS: u32 = 32;
const MAX_SEGMENTS: u32 = 64;

const CoroState = enum(u8) {
    FREE = 0, // slot available for reuse (post-cleanup tombstone)
    PENDING = 1, // created but never resumed
    RUNNING = 2, // body in flight
    SUSPENDED = 3, // yield raised; waiting for next resume
    DEAD = 4, // body returned or errored, awaiting cleanup
};

const Segment = struct {
    src_ptr: u32,
    src_len: u32,
};

pub const Coro = struct {
    name_ptr: u32,
    name_len: u32,
    body_ptr: u32,
    body_len: u32,
    body_obj: i32, // retained TclObj backing body bytes
    /// Namespace + simple-name span of the dispatch command we
    /// registered in :func:`cmds.coroutine.eval_coroutine`.  Held
    /// here so :func:`cleanup_terminated` can clear the
    /// ``cmd_table`` entry without re-resolving the name (the
    /// caller may be on a different namespace by then).  Zero on
    /// FREE slots.
    ns_addr: u32,
    cmd_simple_ptr: u32,
    cmd_simple_len: u32,
    // v1 segment state — unused under asyncify.
    segments: [MAX_SEGMENTS]Segment,
    n_segments: u32,
    next_segment: u32,
    // Stage-2 asyncify state — buffer holds the saved call stack
    // between yield and resume.  Allocated at fixed
    // ``DEFAULT_BUFFER_SIZE`` (16 KB) on the first resume; the
    // current driver does NOT grow the buffer on overflow — an
    // unwind that exceeds the buffer would trap inside the
    // asyncify-generated save code.  Tracked as a Stage 2.5
    // follow-up alongside the multi-yield rewind issue.
    async_buf: u32,
    async_buf_size: u32,
    state: CoroState,
    /// Phase 10: per-coro saved frame context.  Save-transfer
    /// ownership: the coro's frame retains live in this snapshot
    /// while the coro is suspended; on resume we restore them and
    /// drain the caller's live state into ``ctx_caller_save`` for
    /// the duration of the coro's run.  Empty when the coro has
    /// never resumed (first resume starts with a fresh frame
    /// stack).
    ctx: tcl_frames.FrameContext,
    has_ctx: u8,
};

var g_coros: [MAX_COROS]Coro = undefined;
var g_n_coros: u32 = 0;
/// Stack of currently-running coro indices (highest is innermost).
/// :func:`yield` consults the top to decide where to send its value.
/// Empty ⇒ yield is illegal (raise error).
var g_call_stack: [MAX_COROS]u32 = undefined;
var g_call_depth: u32 = 0;

pub fn reset() void {
    g_n_coros = 0;
    g_call_depth = 0;
    tcl_catch.state.yield_flag = 0;
    tcl_catch.state.yield_value = 0;
}

fn name_eq(c: *const Coro, ptr: u32, len: u32) bool {
    if (c.name_len != len) return false;
    const a: [*]const u8 = @ptrFromInt(c.name_ptr);
    const b: [*]const u8 = @ptrFromInt(ptr);
    var i: u32 = 0;
    while (i < len) : (i += 1) if (a[i] != b[i]) return false;
    return true;
}

pub fn lookup(name_ptr: u32, name_len: u32) ?*Coro {
    var i: u32 = 0;
    while (i < g_n_coros) : (i += 1) {
        if (g_coros[i].state == .FREE) continue;
        if (name_eq(&g_coros[i], name_ptr, name_len)) return &g_coros[i];
    }
    return null;
}

/// Split ``body`` into top-level command spans (v1 fallback only).
/// Mirrors a tiny subset of the Tcl tokeniser: tracks brace nesting
/// and bracket nesting, treats unescaped semicolons and newlines at
/// nesting depth zero as command separators.  Backslash-newline
/// counts as whitespace.  Quotes don't suppress separators in real
/// Tcl either.
fn split_segments(c: *Coro) void {
    const src: [*]const u8 = @ptrFromInt(c.body_ptr);
    var i: u32 = 0;
    var seg_start: u32 = 0;
    var brace: u32 = 0;
    var bracket: u32 = 0;
    var in_word = false;
    while (i < c.body_len) : (i += 1) {
        const ch = src[i];
        if (ch == '\\' and i + 1 < c.body_len) {
            i += 1;
            in_word = true;
            continue;
        }
        if (ch == '{') { brace += 1; in_word = true; continue; }
        if (ch == '}') { if (brace > 0) brace -= 1; in_word = true; continue; }
        if (ch == '[') { bracket += 1; in_word = true; continue; }
        if (ch == ']') { if (bracket > 0) bracket -= 1; in_word = true; continue; }
        if (brace > 0 or bracket > 0) { in_word = true; continue; }
        if (ch == ';' or ch == '\n') {
            if (in_word and c.n_segments < MAX_SEGMENTS) {
                c.segments[c.n_segments] = .{
                    .src_ptr = c.body_ptr + seg_start,
                    .src_len = i - seg_start,
                };
                c.n_segments += 1;
            }
            seg_start = i + 1;
            in_word = false;
            continue;
        }
        if (ch != ' ' and ch != '\t') in_word = true;
    }
    if (in_word and c.n_segments < MAX_SEGMENTS) {
        c.segments[c.n_segments] = .{
            .src_ptr = c.body_ptr + seg_start,
            .src_len = c.body_len - seg_start,
        };
        c.n_segments += 1;
    }
}

/// Find an unused slot — first a FREE tombstone, otherwise extend
/// up to MAX_COROS.  Returns null when the table is full.
fn alloc_slot() ?*Coro {
    var i: u32 = 0;
    while (i < g_n_coros) : (i += 1) {
        if (g_coros[i].state == .FREE) return &g_coros[i];
    }
    if (g_n_coros >= MAX_COROS) return null;
    const c = &g_coros[g_n_coros];
    g_n_coros += 1;
    return c;
}

/// Register a new coroutine.  ``name`` is the command name to
/// register.  ``body`` is the script the dispatcher will run.
/// Returns null on table overflow or duplicate-name.
pub fn create(name_ptr: u32, name_len: u32, body_obj: i32) ?*Coro {
    if (lookup(name_ptr, name_len)) |_| return null; // duplicate
    const c = alloc_slot() orelse return null;
    const body_s = obj_ensure_string(body_obj);
    tcl_obj_retain(body_obj);
    // Heap-copy the name so the coroutine survives the caller's
    // word release.
    const nbuf = obj.alloc(name_len);
    if (name_len > 0) obj.memcpy(nbuf, name_ptr, name_len);
    c.name_ptr = nbuf;
    c.name_len = name_len;
    c.body_ptr = body_s.ptr;
    c.body_len = body_s.len;
    c.body_obj = body_obj;
    c.ns_addr = 0;
    c.cmd_simple_ptr = 0;
    c.cmd_simple_len = 0;
    c.n_segments = 0;
    c.next_segment = 0;
    c.async_buf = 0;
    c.async_buf_size = 0;
    c.state = .PENDING;
    // Phase 10: ``ctx`` starts unpopulated.  ``resume_segments``
    // detects the first-resume case via ``has_ctx == 0`` and
    // restores a fresh frame stack instead of an old snapshot.
    c.ctx = std.mem.zeroes(tcl_frames.FrameContext);
    c.has_ctx = 0;
    if (!tcl_async.ENABLED) split_segments(c);
    return c;
}

/// Record where the dispatch command is registered so
/// :func:`cleanup_terminated` can clear it on terminal return.
/// Called by the command-side wrapper right after ``ns_cmd_put``.
/// Heap-copies the simple-name bytes — the caller's pointer is
/// usually into a parser word buffer that gets released long
/// before the coroutine terminates.
pub fn record_registration(c: *Coro, ns_addr: u32, simple_ptr: u32, simple_len: u32) void {
    c.ns_addr = ns_addr;
    if (simple_len > 0) {
        const buf = obj.alloc(simple_len);
        obj.memcpy(buf, simple_ptr, simple_len);
        c.cmd_simple_ptr = buf;
    } else {
        c.cmd_simple_ptr = 0;
    }
    c.cmd_simple_len = simple_len;
}

/// Tear down a DEAD coroutine: clear its namespace command so a
/// later ``[name]`` call surfaces a clean ``invalid command name``
/// error instead of returning empty, release the retained body
/// TclObj, and mark the slot FREE so the next ``create`` can
/// reuse it.  Codex P1 on PR #284: completed coros previously
/// accumulated permanently in the fixed MAX_COROS table.
pub fn cleanup_terminated(c: *Coro) void {
    if (c.state != .DEAD) return;
    if (c.ns_addr != 0 and c.cmd_simple_len != 0) {
        const tcl_ns = @import("../interp/tcl_ns.zig");
        _ = tcl_ns.ns_cmd_clear(c.ns_addr, c.cmd_simple_ptr, c.cmd_simple_len);
        // ``proc_lookup`` caches the resolved Command pointer; the
        // freshly-cleared bucket is fine but a previously-cached
        // pointer to our dispatch Command would still survive.
        // Drop the cache so the next ``[name]`` call re-resolves
        // and sees the tombstone.
        const tcl_procs = @import("../interp/tcl_procs.zig");
        tcl_procs.lru_invalidate_all();
    }
    if (c.body_obj != 0) tcl_obj_release(c.body_obj);
    c.body_obj = 0;
    c.ns_addr = 0;
    c.cmd_simple_ptr = 0;
    c.cmd_simple_len = 0;
    c.state = .FREE;
}

fn push_call(c: *Coro) bool {
    if (g_call_depth >= MAX_COROS) return false;
    const idx: u32 = @intCast((@intFromPtr(c) - @intFromPtr(&g_coros[0])) / @sizeOf(Coro));
    g_call_stack[g_call_depth] = idx;
    g_call_depth += 1;
    return true;
}

fn pop_call() void {
    if (g_call_depth > 0) g_call_depth -= 1;
}

fn current_coro() ?*Coro {
    if (g_call_depth == 0) return null;
    return &g_coros[g_call_stack[g_call_depth - 1]];
}

/// Stage-2 asyncify resume — must be EXCLUDED from asyncify
/// instrumentation via ``--pass-arg=asyncify-removelist@tcl_coro_drive``
/// so the unwind triggered by yield stops here instead of
/// propagating all the way out to the host.  Exposed as
/// ``pub export fn`` so the export name survives the linker's
/// symbol stripping; without an export the function index is
/// anonymous and asyncify can't find it by name in the removelist.
///
/// Either invokes the body for the first time or rewinds the saved
/// stack so execution resumes inside the previous yield call.  When
/// the body suspends via yield this function returns the yielded
/// value; when the body returns normally we report the body's result
/// and mark the coroutine DEAD.
pub export fn tcl_coro_drive(coro_addr: i32) i32 {
    const c: *Coro = @ptrFromInt(@as(u32, @bitCast(coro_addr)));
    const interp = @import("../interp/tcl_interp.zig");

    if (c.state == .DEAD) return 0;

    const is_resume = c.state == .SUSPENDED;
    if (c.state == .PENDING) {
        c.async_buf = obj.alloc(tcl_async.DEFAULT_BUFFER_SIZE);
        c.async_buf_size = tcl_async.DEFAULT_BUFFER_SIZE;
        tcl_async.init_buffer(c.async_buf, c.async_buf_size);
    }

    // Every instrumented helper call MUST happen before
    // ``asyncify_start_rewind`` — otherwise the helper's prelude
    // sees the rewind state, tries to restore from a buffer that
    // never held its frame, and traps ``unreachable`` on the
    // bogus state-machine ID.  Asyncify treats *import* calls as
    // leaves (no instrumentation), so ``start_rewind`` itself plus
    // the inline memory writes below are safe right up to the
    // ``eval_script`` re-entry.
    if (!push_call(c)) return 0;
    // Note: ``defer pop_call();`` would inject pop_call into the
    // unwind path, which under asyncify writes its frame into the
    // saved buffer.  Replicate it by hand AFTER eval_script
    // returns instead.

    c.state = .RUNNING;
    tcl_catch.state.yield_flag = 0;
    tcl_catch.state.yield_value = 0;

    if (is_resume) {
        // Last instrumented call is push_call above; from here to
        // eval_script we touch only imports + raw memory.
        tcl_async.asyncify_start_rewind(c.async_buf);
    }

    const result = interp.eval_script(c.body_ptr, c.body_len);

    // After eval_script returns, three things can have happened:
    //   1. Body completed normally — coroutine is now DEAD.
    //   2. yield() called ``asyncify_start_unwind``, the eval stack
    //      unwound, control returned here through that mechanism.
    //      ``asyncify_get_state`` reports UNWINDING; we stop the
    //      unwind so the next resume can either rewind or start
    //      fresh.
    //   3. error / return propagated up; treat like (1).
    const state = tcl_async.asyncify_get_state();
    if (state == tcl_async.STATE_UNWINDING) {
        tcl_async.asyncify_stop_unwind();
        const yv = tcl_catch.state.yield_value;
        tcl_catch.state.yield_flag = 0;
        tcl_catch.state.yield_value = 0;
        c.state = .SUSPENDED;
        // pop_call() runs while state is NORMAL — safe (no rewind
        // active).  But to keep the semantics of the v1 driver
        // (g_call_depth = 0 outside an active resume) we MUST pop
        // before returning.  Inline the body to dodge a function
        // call on the unwind path; the asyncify pass would try to
        // save/restore it otherwise.
        if (g_call_depth > 0) g_call_depth -= 1;
        return yv;
    }
    if (state == tcl_async.STATE_REWINDING) {
        tcl_async.asyncify_stop_rewind();
    }
    if (g_call_depth > 0) g_call_depth -= 1;
    c.state = .DEAD;
    return result;
}

fn resume_async(c: *Coro) i32 {
    return tcl_coro_drive(@bitCast(@intFromPtr(c)));
}

/// v1 segment-based resume — used when asyncify is disabled.
///
/// Phase 10: each coro now carries its own frame stack via
/// :type:`tcl_frames.FrameContext`.  On entry we save-transfer the
/// caller's live frames into a local ``caller_ctx`` and restore the
/// coro's previously-saved snapshot (or a fresh stack on first
/// resume).  On yield we save-transfer the coro's frames back into
/// its slot and restore the caller's.  On terminal completion the
/// caller's frames come back; the coro's snapshot is dropped (the
/// retains it owned die with it, which is correct since the coro
/// is terminating).
fn resume_segments(c: *Coro) i32 {
    if (c.state == .DEAD) return 0;
    if (c.next_segment >= c.n_segments) {
        c.state = .DEAD;
        return 0;
    }
    const interp = @import("../interp/tcl_interp.zig");
    if (!push_call(c)) return 0;
    defer pop_call();

    // Save-transfer the caller's frame state.  ``frame_context_save_transfer``
    // moves the live retains into the snapshot and zeroes the
    // live slots so the coro starts with a clean stack.
    const caller_ctx = tcl_frames.frame_context_save_transfer();

    if (c.has_ctx != 0) {
        // Restore the coro's previously-saved frames.  Symmetric
        // transfer — no retains/releases happen, just bytes copy in.
        tcl_frames.frame_context_restore_transfer(c.ctx);
    }
    // First resume: live state is already empty after the save above,
    // so the coro starts with a fresh frame_depth=0.

    c.state = .RUNNING;
    tcl_catch.state.yield_flag = 0;
    tcl_catch.state.yield_value = 0;

    var result: i32 = 0;
    while (c.next_segment < c.n_segments) {
        const seg = c.segments[c.next_segment];
        c.next_segment += 1;
        if (seg.src_len == 0) continue;
        result = interp.eval_script(seg.src_ptr, seg.src_len);
        if (tcl_catch.state.yield_flag != 0) {
            tcl_catch.state.yield_flag = 0;
            const yv = tcl_catch.state.yield_value;
            tcl_catch.state.yield_value = 0;
            c.state = .SUSPENDED;
            // Save-transfer the coro's frames into its slot, then
            // restore the caller's previously-saved context.  The
            // coro's retains now live in ``c.ctx``; the caller's
            // live state goes back to whatever it had before resume.
            c.ctx = tcl_frames.frame_context_save_transfer();
            c.has_ctx = 1;
            tcl_frames.frame_context_restore_transfer(caller_ctx);
            return yv;
        }
        const ir = result_mod.snapshot(result);
        if (ir.code == .ERROR or ir.code == .RETURN) {
            // Consume the signal — the coroutine is terminating with
            // this segment's result as its ``[c]`` return value.
            // Without consuming, the RETURN flag leaks past the
            // coroutine boundary and any subsequent ``catch`` /
            // ``has_signal`` probe at script level mistakes the
            // coroutine's terminal state for an in-flight return,
            // tagging the next command's catch result with rc=2 and
            // the coroutine's return value (regression caught by
            // ``test_coroutine_command_removed_after_terminal_return``).
            // ERROR similarly: if the coroutine body errored, the
            // error should be the coroutine's terminal observable
            // result; the surrounding caller decides whether to
            // forward it (compiled-proc dispatch) or swallow it
            // (background scheduler).  Today the only consumer is
            // ``puts [c]`` which propagates the i32 result by
            // value, so consuming here keeps the global state
            // clean for the next top-level command.
            if (ir.code == .RETURN) result_mod.consume(.RETURN);
            // ERROR is left set so the caller still observes it —
            // the coroutine's command-table tombstone clears on
            // ``cleanup_terminated`` so a follow-up ``[c]`` correctly
            // surfaces ``invalid command name``, but the body's
            // original error should still propagate this round.
            c.state = .DEAD;
            // Drop the coro's frames (they're terminating) and
            // restore the caller's.  ``cleanup_terminated`` will
            // run next; the snapshot in ``c.ctx`` becomes
            // unreachable along with the slot.
            tcl_frames.frame_context_restore_transfer(caller_ctx);
            c.has_ctx = 0;
            return result;
        }
    }
    c.state = .DEAD;
    // Body ran to completion without yield — same teardown as the
    // ERROR/RETURN path: drop coro frames, restore caller.
    tcl_frames.frame_context_restore_transfer(caller_ctx);
    c.has_ctx = 0;
    return result;
}

/// Resume the coroutine.  Routes to the asyncify driver when
/// available, segment driver otherwise.  When the underlying
/// driver flips state to DEAD (terminal return / error) we tear
/// down the dispatch command and free the slot so subsequent
/// ``[name]`` calls surface ``invalid command name`` instead of
/// silently returning empty, and so the fixed MAX_COROS table
/// doesn't accumulate tombstones forever (Codex P1 on PR #284).
pub fn resume_one(c: *Coro) i32 {
    const result = if (tcl_async.ENABLED) resume_async(c) else resume_segments(c);
    if (c.state == .DEAD) cleanup_terminated(c);
    return result;
}

/// Called by the ``yield`` command.  Signals the coroutine driver
/// to suspend and propagate ``value`` back to the caller of ``[c]``.
/// When called outside a coroutine, returns false so the caller
/// emits a ``yield without coroutine`` error.  Under the v1 driver
/// this just sets ``yield_flag`` and lets the segment loop notice;
/// under asyncify we route the unwind through
/// ``tcl_async.coro_yield_unwind`` — a host-trampolined import
/// listed in ``--pass-arg=asyncify-imports`` so the asyncify pass
/// wraps callers' calls with the standard ``state==UNWINDING ⇒
/// save+br`` epilogue.  The host trampoline routes on the asyncify
/// state: ``NORMAL`` ⇒ ``asyncify_start_unwind`` (begin unwind);
/// ``REWINDING`` ⇒ ``asyncify_stop_rewind`` (transition to NORMAL
/// so the body resumes past the original ``yield`` to the next
/// one).  Calling ``asyncify_start_unwind`` directly from here
/// would loop on resume because the asyncify rewind dispatches to
/// the saved call site and re-fires the unwind (issue #282).
pub fn signal_yield(value: i32) bool {
    if (g_call_depth == 0) return false;
    if (value != 0) tcl_obj_retain(value);
    tcl_catch.state.yield_flag = 1;
    tcl_catch.state.yield_value = value;
    if (tcl_async.ENABLED) {
        if (current_coro()) |c| {
            tcl_async.coro_yield_unwind(c.async_buf);
        }
    }
    return true;
}

pub fn current_in_coroutine() bool {
    return g_call_depth > 0;
}
