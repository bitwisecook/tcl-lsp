// Coroutine support — v1 (segment-based, Stage 1).
//
// Real Tcl coroutines suspend at arbitrary call depth via NRE.  Without
// NRE or asyncify (Stage 2 in the event-loop staircase) we can't
// resume execution mid-callstack.  v1 ships a *severely* restricted
// model that nonetheless covers the common test idiom:
//
//     coroutine c apply {{} { yield A; yield B; return C }}
//     [c]   ;# A
//     [c]   ;# B
//     [c]   ;# C
//
// Mechanism:
//
//   1. ``coroutine NAME prefix args...`` immediately invokes
//      ``prefix args...`` exactly once, with a marker pushed on the
//      coro stack.  The marker tells :func:`yield` to capture its
//      argument and raise a ``SIG_YIELD`` "signal" that unwinds back
//      to the coroutine driver.
//   2. The driver registers ``NAME`` as a CMD_COROUTINE redirect
//      Command.  Calling it invokes :func:`resume`, which evaluates
//      the *next* segment of the body.
//   3. Segments are computed by parsing the body once at coro
//      creation: top-level command boundaries become candidate
//      resumption points.  Concretely, the body is split into a
//      list of (start, len) source-byte spans; each call to ``c``
//      eats and evaluates the next span.  ``yield`` short-circuits
//      the segment so subsequent commands in the same span run on
//      the next resume.
//
// Limitations (acceptable for v1, documented in
// ``docs/design/runtime/event-loop.md``):
//
//   * ``yield`` only works when invoked as a top-level command in
//     the coroutine body.  ``set x [yield]`` or yields inside a
//     nested ``proc`` raise an error.
//   * No locals persistence between segments — each segment runs
//     in a fresh frame.  Workaround: use ``set ::ns::var`` to
//     persist state.  Stage 2 with asyncify removes this limit.
//   * ``yieldto`` invokes the target command and yields its result;
//     no restriction beyond the above.

const obj = @import("../valtypes/tcl_obj.zig");
const tcl_obj_retain = obj.tcl_obj_retain;
const tcl_obj_release = obj.tcl_obj_release;
const obj_new_string = obj.obj_new_string;
const obj_ensure_string = obj.obj_ensure_string;

const tcl_catch = @import("../interp/tcl_catch.zig");

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
    segments: [MAX_SEGMENTS]Segment,
    n_segments: u32,
    next_segment: u32,
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

/// Split ``body`` into top-level command spans.  Mirrors a tiny
/// subset of the Tcl tokeniser: tracks brace nesting and bracket
/// nesting, treats unescaped semicolons and newlines at nesting
/// depth zero as command separators.  Backslash-newline counts as
/// whitespace (the line continuation form).  Quotes don't suppress
/// command separators in real Tcl either.
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
/// register.  ``body`` is the script (typically ``apply {{} { ... }}``
/// produced by the caller's word-concatenation).  Returns null on
/// table overflow.  Stage-2's asyncify will drop the segment array
/// and just run the body straight through.
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
    c.state = .PENDING;
    split_segments(c);
    return c;
}

/// Resume the coroutine: evaluate the next segment.  Returns the
/// yield value (or the segment's return value if the segment
/// finished without yielding).  When all segments have run, the
/// coroutine becomes DEAD and subsequent resume calls return an
/// error.
pub fn resume_one(c: *Coro) i32 {
    if (c.state == .DEAD) return 0;
    if (c.next_segment >= c.n_segments) {
        c.state = .DEAD;
        return 0;
    }
    const interp = @import("../interp/tcl_interp.zig");
    // Push call-stack so a top-level yield knows which coro to
    // attribute its value to.
    if (g_call_depth >= MAX_COROS) return 0;
    const idx: u32 = @intCast((@intFromPtr(c) - @intFromPtr(&g_coros[0])) / @sizeOf(Coro));
    g_call_stack[g_call_depth] = idx;
    g_call_depth += 1;
    defer g_call_depth -= 1;

    c.state = .RUNNING;
    tcl_catch.yield_flag = 0;
    tcl_catch.yield_value = 0;

    var result: i32 = 0;
    // Evaluate segments until one of: yield, error, return, or all
    // segments consumed.
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

/// Called by the ``yield`` command.  Signals the coroutine driver
/// to stop the current segment and propagate ``value`` back to the
/// caller of ``[c]``.  When called outside a coroutine, sets
/// ``error_flag`` with a "yield without coroutine" message and
/// returns false.
pub fn signal_yield(value: i32) bool {
    if (g_call_depth == 0) return false;
    if (value != 0) tcl_obj_retain(value);
    tcl_catch.yield_flag = 1;
    tcl_catch.yield_value = value;
    return true;
}

pub fn current_in_coroutine() bool {
    return g_call_depth > 0;
}
