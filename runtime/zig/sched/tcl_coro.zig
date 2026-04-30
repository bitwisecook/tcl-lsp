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
// **Status of the asyncify path (Stage 2 partial):**
//
//   * Single-yield works correctly — yield from arbitrary call
//     depth (apply / proc / nested loops) unwinds cleanly to the
//     driver and returns the yielded value.
//   * Multi-yield rewind has a known issue: the second ``[c]`` call
//     either returns empty or stack-exhausts depending on the
//     surrounding shape.  Suspected interaction between the rewind
//     state machine and the proc-dispatch / apply-frame paths.
//     Tracked as Stage 2.5; the v1 segment driver is selected by
//     default so this doesn't affect production use.
//
// Both drivers share the public surface (:func:`create`,
// :func:`lookup`, :func:`resume_one`, :func:`signal_yield`,
// :func:`current_in_coroutine`) so the dispatch site in
// :func:`eval_proc_call_bucket` doesn't care which model is active.

const obj = @import("../valtypes/tcl_obj.zig");
const tcl_obj_retain = obj.tcl_obj_retain;
const tcl_obj_release = obj.tcl_obj_release;
const obj_new_string = obj.obj_new_string;
const obj_ensure_string = obj.obj_ensure_string;

const tcl_catch = @import("../interp/tcl_catch.zig");
const tcl_async = @import("tcl_asyncify.zig");

// The yield signal flag lives on ``tcl_catch.yield_flag`` so
// ``has_signal()`` in the interpreter sees it without a module-
// circular import.  Reads/writes go through tcl_catch directly.

const MAX_COROS: u32 = 32;
const MAX_SEGMENTS: u32 = 64;

const CoroState = enum(u8) {
    PENDING = 0, // never been resumed
    RUNNING = 1, // body in flight
    SUSPENDED = 2, // yield raised; waiting for next resume
    DEAD = 3, // body returned or errored
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
    // v1 segment state — unused under asyncify.
    segments: [MAX_SEGMENTS]Segment,
    n_segments: u32,
    next_segment: u32,
    // Stage-2 asyncify state — buffer holds the saved call stack
    // between yield and resume.  ``buf == 0`` until the first
    // resume allocates it; thereafter it persists for the
    // coroutine's lifetime.
    async_buf: u32,
    async_buf_size: u32,
    state: CoroState,
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
    tcl_catch.yield_flag = 0;
    tcl_catch.yield_value = 0;
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

/// Register a new coroutine.  ``name`` is the command name to
/// register.  ``body`` is the script the dispatcher will run.
/// Returns null on table overflow.
pub fn create(name_ptr: u32, name_len: u32, body_obj: i32) ?*Coro {
    if (g_n_coros >= MAX_COROS) return null;
    if (lookup(name_ptr, name_len)) |_| return null; // duplicate
    const body_s = obj_ensure_string(body_obj);
    tcl_obj_retain(body_obj);
    // Heap-copy the name so the coroutine survives the caller's
    // word release.
    const nbuf = obj.alloc(name_len);
    if (name_len > 0) obj.memcpy(nbuf, name_ptr, name_len);
    const c = &g_coros[g_n_coros];
    g_n_coros += 1;
    c.name_ptr = nbuf;
    c.name_len = name_len;
    c.body_ptr = body_s.ptr;
    c.body_len = body_s.len;
    c.body_obj = body_obj;
    c.n_segments = 0;
    c.next_segment = 0;
    c.async_buf = 0;
    c.async_buf_size = 0;
    c.state = .PENDING;
    if (!tcl_async.ENABLED) split_segments(c);
    return c;
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
    tcl_catch.yield_flag = 0;
    tcl_catch.yield_value = 0;

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
        const yv = tcl_catch.yield_value;
        tcl_catch.yield_flag = 0;
        tcl_catch.yield_value = 0;
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
fn resume_segments(c: *Coro) i32 {
    if (c.state == .DEAD) return 0;
    if (c.next_segment >= c.n_segments) {
        c.state = .DEAD;
        return 0;
    }
    const interp = @import("../interp/tcl_interp.zig");
    if (!push_call(c)) return 0;
    defer pop_call();

    c.state = .RUNNING;
    tcl_catch.yield_flag = 0;
    tcl_catch.yield_value = 0;

    var result: i32 = 0;
    while (c.next_segment < c.n_segments) {
        const seg = c.segments[c.next_segment];
        c.next_segment += 1;
        if (seg.src_len == 0) continue;
        result = interp.eval_script(seg.src_ptr, seg.src_len);
        if (tcl_catch.yield_flag != 0) {
            tcl_catch.yield_flag = 0;
            const yv = tcl_catch.yield_value;
            tcl_catch.yield_value = 0;
            c.state = .SUSPENDED;
            return yv;
        }
        if (tcl_catch.error_flag != 0 or tcl_catch.return_flag != 0) {
            c.state = .DEAD;
            return result;
        }
    }
    c.state = .DEAD;
    return result;
}

/// Resume the coroutine.  Routes to the asyncify driver when
/// available, segment driver otherwise.
pub fn resume_one(c: *Coro) i32 {
    if (tcl_async.ENABLED) return resume_async(c);
    return resume_segments(c);
}

/// Called by the ``yield`` command.  Signals the coroutine driver
/// to suspend and propagate ``value`` back to the caller of ``[c]``.
/// When called outside a coroutine, returns false so the caller
/// emits a ``yield without coroutine`` error.  Under asyncify we
/// trigger the unwind here; under the v1 driver we just set
/// ``yield_flag`` and let the segment loop notice.
pub fn signal_yield(value: i32) bool {
    if (g_call_depth == 0) return false;
    if (value != 0) tcl_obj_retain(value);
    tcl_catch.yield_flag = 1;
    tcl_catch.yield_value = value;
    if (tcl_async.ENABLED) {
        if (current_coro()) |c| {
            tcl_async.asyncify_start_unwind(c.async_buf);
        }
    }
    return true;
}

pub fn current_in_coroutine() bool {
    return g_call_depth > 0;
}
