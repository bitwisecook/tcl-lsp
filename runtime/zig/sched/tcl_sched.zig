// Central event-loop scheduler for the WASM Tcl runtime.
//
// Owns:
//   * the timer heap (``after MS script``)
//   * the idle queue (``after idle script``)
//   * the fileevent registration table (registration only in v1)
//   * the active vwait stack (tells the var-write hook which target
//     names trip a wake)
//   * a monotonic ``after#N`` id counter
//
// External shape:
//   * :func:`schedule_after` / :func:`schedule_idle` — register new
//     scripts; return the ``after#N`` id as a TclObj.
//   * :func:`cancel_by_id` / :func:`cancel_by_script` — back out
//     pending scripts by id or script body.
//   * :func:`info_all` — list pending ids as a TclObj list (``after
//     info``).
//   * :func:`info_one` — describe one pending entry as ``{script
//     timer|idle}``.
//   * :func:`drain_all` — used by ``update``.
//   * :func:`drain_idle` — used by ``update idletasks``.
//   * :func:`vwait_drain` — used by ``vwait``: dispatch events until
//     the named variable is written or the queue is empty.
//
// All script dispatch routes through :func:`run_script_obj`, which
// invokes ``interp.eval_script`` with the body's TclObj string
// representation.  Errors raised inside scheduled scripts swallow
// the ``error_flag`` so the drain loop can continue (Stage 4 will
// route them through ``bgerror`` instead).

const std = @import("std");
const obj = @import("../valtypes/tcl_obj.zig");
const tcl_obj_retain = obj.tcl_obj_retain;
const tcl_obj_release = obj.tcl_obj_release;
const obj_new_string = obj.obj_new_string;
const obj_new_int = obj.obj_new_int;
const obj_ensure_string = obj.obj_ensure_string;

const timer_mod = @import("tcl_timer.zig");
const ready_mod = @import("tcl_ready.zig");
const fevent_mod = @import("tcl_fileevent.zig");
const vwait_mod = @import("tcl_vwait.zig");
const tcl_clock = @import("../io/tcl_clock.zig");
const tcl_catch = @import("../interp/tcl_catch.zig");
const result_mod = @import("../interp/tcl_result.zig");

// Global scheduler state.  Single-instance because the WASM runtime
// is single-interp today; when ``interp::create`` lands, this struct
// gets hung off the active Interp.
var g_timers: timer_mod.Heap = .{};
var g_idle: ready_mod.Queue = .{};
var g_fileevents: fevent_mod.Table = .{};
var g_vwait: vwait_mod.Stack = .{};
var g_next_id: u32 = 1;

/// Reset all scheduler state.  Used at startup and by tests that
/// need a clean slate between cases.
pub fn reset() void {
    g_timers = .{};
    g_idle = .{};
    g_fileevents = .{};
    g_vwait = .{};
    g_next_id = 1;
}

/// Number of pending entries across the timer heap + idle queue.
/// Fileevent handlers are intentionally NOT counted: Stage 1 never
/// fires them so they're not "events pending" — the real Tcl shape
/// folds them in once Stage 3 wires the WASI ``poll_oneoff`` path.
pub fn pending_count() u32 {
    return g_timers.len + g_idle.count();
}

/// Monotonic clock in microseconds.  Wraps the existing WASI clock
/// helper.  The function is exposed via this module so test
/// scaffolding can trivially shim it later (Stage 6's
/// determinism hook).
pub fn now_us() i64 {
    const ns = tcl_clock.clock_ns(.MONOTONIC);
    return @divTrunc(ns, 1_000);
}

fn alloc_id() u32 {
    const id = g_next_id;
    g_next_id += 1;
    return id;
}

/// Format ``after#N`` into a fresh TclObj string.
fn format_id(id: u32) i32 {
    var buf: [32]u8 = undefined;
    const prefix = "after#";
    var off: u32 = 0;
    for (prefix) |c| {
        buf[off] = c;
        off += 1;
    }
    // Render the number.
    var digits: [10]u8 = undefined;
    var n: u32 = 0;
    var v = id;
    if (v == 0) {
        digits[0] = '0';
        n = 1;
    } else {
        while (v > 0) : (n += 1) {
            digits[n] = @intCast('0' + v % 10);
            v /= 10;
        }
    }
    while (n > 0) {
        n -= 1;
        buf[off] = digits[n];
        off += 1;
    }
    const out = obj.alloc(off);
    const dst: [*]u8 = @ptrFromInt(out);
    var i: u32 = 0;
    while (i < off) : (i += 1) dst[i] = buf[i];
    return obj_new_string(@bitCast(out), @bitCast(off));
}

/// Parse ``after#N``.  Returns the id, or 0 on miss.
fn parse_id(name_ptr: u32, name_len: u32) u32 {
    if (name_len < 7) return 0;
    const prefix = "after#";
    const p: [*]const u8 = @ptrFromInt(name_ptr);
    var i: u32 = 0;
    while (i < prefix.len) : (i += 1) {
        if (p[i] != prefix[i]) return 0;
    }
    var v: u32 = 0;
    while (i < name_len) : (i += 1) {
        const c = p[i];
        if (c < '0' or c > '9') return 0;
        v = v * 10 + (c - '0');
    }
    return v;
}

/// Schedule a ``after MS script``.  ``ms`` may be 0 (fires on the
/// next drain).  Retains the script TclObj so it survives until
/// dispatch (or cancel).  Returns the ``after#N`` id TclObj.
pub fn schedule_after(ms: i64, script_obj: i32) i32 {
    tcl_obj_retain(script_obj);
    const id = alloc_id();
    const deadline = now_us() + ms * 1_000;
    g_timers.push(.{
        .id = id,
        .deadline_us = deadline,
        .script_obj = script_obj,
        .is_idle = 0,
    });
    return format_id(id);
}

/// Schedule a ``after idle script``.  Drained by ``update`` /
/// ``update idletasks`` after timers.
pub fn schedule_idle(script_obj: i32) i32 {
    tcl_obj_retain(script_obj);
    const id = alloc_id();
    g_idle.push(id, script_obj);
    return format_id(id);
}

/// Cancel a pending entry by id.  Returns true on hit.  Releases
/// the cancelled script's TclObj.
pub fn cancel_by_id(id: u32) bool {
    const tscript = g_timers.cancel_by_id(id);
    if (tscript != 0) {
        tcl_obj_release(tscript);
        return true;
    }
    const iscript = g_idle.cancel_by_id(id);
    if (iscript != 0) {
        tcl_obj_release(iscript);
        return true;
    }
    return false;
}

/// Cancel by matching script bytes (any single matching pending
/// entry).  Returns the cancelled id, or 0 on miss.  Releases the
/// removed entry's retained script TclObj — the underlying timer
/// and idle modules return the script handle alongside the id so
/// the leak ``cancel_by_id`` already avoided is closed here too
/// (Codex P2 on PR #284).
pub fn cancel_by_script(script_ptr: u32, script_len: u32) u32 {
    const tres = g_timers.cancel_by_script(script_ptr, script_len);
    if (tres.id != 0) {
        if (tres.script_obj != 0) tcl_obj_release(tres.script_obj);
        return tres.id;
    }
    const ires = g_idle.cancel_by_script(script_ptr, script_len);
    if (ires.id != 0 and ires.script_obj != 0) tcl_obj_release(ires.script_obj);
    return ires.id;
}

/// Cancel by id-or-script: parses the input as ``after#N`` first,
/// then falls back to script-bytes match.  Returns true if anything
/// was cancelled.
pub fn cancel_by_token(token_ptr: u32, token_len: u32) bool {
    const id = parse_id(token_ptr, token_len);
    if (id != 0) {
        if (cancel_by_id(id)) return true;
        // The id parsed but didn't match a pending entry — treat
        // as a no-op (real Tcl semantics for ``after cancel`` of an
        // unknown id).
        return false;
    }
    return cancel_by_script(token_ptr, token_len) != 0;
}

/// Build ``{after#1 after#2 …}`` for ``after info``.
pub fn info_all() i32 {
    // Compute total length first.
    var n_timers: u32 = 0;
    while (g_timers.at(n_timers)) |_| : (n_timers += 1) {}
    var total: u32 = 0;
    var i: u32 = 0;
    while (i < n_timers) : (i += 1) {
        const e = g_timers.at(i).?;
        // ``after#N`` digits + space.  ``digit_count`` for u32 max
        // is 10.
        total += 7 + 10 + 1;
        _ = e;
    }
    var idle_walk: u32 = 0;
    while (g_idle.at(idle_walk)) |_| : (idle_walk += 1) {
        total += 7 + 10 + 1;
    }
    if (total == 0) return obj_new_string(0, 0);
    const buf = obj.alloc(total);
    const dst: [*]u8 = @ptrFromInt(buf);
    var off: u32 = 0;
    i = 0;
    while (i < n_timers) : (i += 1) {
        const e = g_timers.at(i).?;
        off = write_id(dst, off, e.id);
        dst[off] = ' ';
        off += 1;
    }
    idle_walk = 0;
    while (g_idle.at(idle_walk)) |e| : (idle_walk += 1) {
        off = write_id(dst, off, e.id);
        dst[off] = ' ';
        off += 1;
    }
    if (off > 0) off -= 1; // strip trailing space
    return obj_new_string(@bitCast(buf), @bitCast(off));
}

fn write_id(dst: [*]u8, off_in: u32, id: u32) u32 {
    var off = off_in;
    const prefix = "after#";
    for (prefix) |c| {
        dst[off] = c;
        off += 1;
    }
    var digits: [10]u8 = undefined;
    var n: u32 = 0;
    var v = id;
    if (v == 0) { digits[0] = '0'; n = 1; }
    else while (v > 0) : (n += 1) { digits[n] = @intCast('0' + v % 10); v /= 10; }
    while (n > 0) { n -= 1; dst[off] = digits[n]; off += 1; }
    return off;
}

/// ``after info <id>`` — return ``{script type}`` for a single id,
/// or 0 (null TclObj) if the id is unknown.  ``type`` is "timer"
/// (any after with ms ≥ 0) or "idle".
pub fn info_one(token_ptr: u32, token_len: u32) i32 {
    const id = parse_id(token_ptr, token_len);
    if (id == 0) return 0;
    var i: u32 = 0;
    while (g_timers.at(i)) |e| : (i += 1) {
        if (e.id == id) return build_info_pair(e.script_obj, "timer");
    }
    i = 0;
    while (g_idle.at(i)) |e| : (i += 1) {
        if (e.id == id) return build_info_pair(e.script_obj, "idle");
    }
    return 0;
}

fn build_info_pair(script_obj: i32, kind: []const u8) i32 {
    const s = obj_ensure_string(script_obj);
    // Result shape: ``script type`` rendered as a real Tcl list so
    // ``lindex`` round-trips.  Naive ``{...}`` wrap broke for
    // scripts with unmatched ``}`` bytes (e.g. ``after 100 "puts }"``)
    // — Codex P1 on PR #284.  Use ``obj.list_elem_quote`` which
    // brace-wraps when safe, escapes otherwise, and falls back to
    // backslash-quoting when an unmatched brace would split the
    // word.  Worst-case expansion for the script element is ~2x
    // (every byte becomes ``\X``); plus 1 byte for the separator
    // and ``kind.len`` for the trailing word.
    const cap: u32 = s.len * 2 + 8 + 1 + @as(u32, @intCast(kind.len));
    const buf = obj.alloc(cap);
    var off: u32 = 0;
    off = obj.list_elem_quote(buf, off, s.ptr, s.len);
    const dst: [*]u8 = @ptrFromInt(buf);
    dst[off] = ' '; off += 1;
    for (kind) |c| { dst[off] = c; off += 1; }
    return obj_new_string(@bitCast(buf), @bitCast(off));
}

// -- Dispatch -----------------------------------------------------------------

fn run_script_obj(script_obj: i32) void {
    const interp = @import("../interp/tcl_interp.zig");
    const s = obj_ensure_string(script_obj);
    if (s.len == 0) return;
    // Save / clear the four signal flags AND their associated payload
    // globals around the dispatch so an error / return raised inside
    // a scheduled script doesn't poison subsequent events on the same
    // drain.  Without snapshotting ``error_msg`` / ``return_val`` /
    // ``yield_value`` alongside the flags, a payload TclObj produced
    // by the scheduled script would leak (we'd overwrite the slot
    // when restoring the flag without releasing).  Stage 4 will route
    // errors through ``bgerror``; for v1 we just swallow them after
    // writing to stderr.  Codex / Copilot review on PR #284.
    const snap = result_mod.signal_save_and_clear();
    _ = interp.eval_script(s.ptr, s.len);
    if (result_mod.snapshot(0).code == .ERROR) {
        // Write a minimal background-error marker to stderr so the
        // failure isn't silent.  Stage 4 promotes this to a real
        // ``bgerror`` invocation.
        const stderr_msg = "bgerror: error in scheduled script\n";
        var iov: [1]std.os.wasi.ciovec_t = undefined;
        iov[0] = .{
            .base = stderr_msg.ptr,
            .len = stderr_msg.len,
        };
        var written: usize = undefined;
        _ = std.os.wasi.fd_write(2, &iov, 1, &written);
    }
    // Release any payload TclObjs the scheduled script left behind so
    // they don't leak when we discard the signal.  Snapshot here
    // captures the post-eval payloads (without clearing them), so we
    // can release through the live slots before the restore stamps
    // back the caller's saved state.
    if (tcl_catch.error_msg != 0) tcl_obj_release(tcl_catch.error_msg);
    if (tcl_catch.return_val != 0) tcl_obj_release(tcl_catch.return_val);
    if (tcl_catch.yield_value != 0) tcl_obj_release(tcl_catch.yield_value);
    result_mod.signal_restore(snap);
}

/// Drain one timer that's due (deadline ≤ now), if any.  Returns
/// true if a timer fired.
fn try_drain_one_timer() bool {
    const now = now_us();
    if (g_timers.pop_if_due(now)) |e| {
        run_script_obj(e.script_obj);
        tcl_obj_release(e.script_obj);
        return true;
    }
    return false;
}

/// Drain one idle script if any are queued.
fn try_drain_one_idle() bool {
    if (g_idle.pop()) |e| {
        run_script_obj(e.script_obj);
        tcl_obj_release(e.script_obj);
        return true;
    }
    return false;
}

/// ``update idletasks`` — drain only idle scripts.  Returns the
/// number of scripts dispatched.
pub fn drain_idle() u32 {
    var n: u32 = 0;
    while (try_drain_one_idle()) : (n += 1) {}
    return n;
}

/// ``update`` — drain every event currently ready: ready timers
/// (deadline ≤ now), then idle scripts.  Repeats until nothing is
/// pending.  Does NOT block waiting for a future timer.  Returns the
/// number of scripts dispatched.
pub fn drain_all() u32 {
    var n: u32 = 0;
    while (true) {
        const before = n;
        while (try_drain_one_timer()) : (n += 1) {}
        while (try_drain_one_idle()) : (n += 1) {}
        if (n == before) break;
    }
    return n;
}

/// Sleep until ``deadline_us`` (monotonic-µs) using WASI's
/// ``poll_oneoff`` on a single clock subscription.  We use this
/// instead of busy-looping inside ``vwait`` so wasmtime actually
/// yields to the host.
fn sleep_until(deadline_us: i64) void {
    const now = now_us();
    if (deadline_us <= now) return;
    const ns: u64 = @intCast((deadline_us - now) * 1_000);
    var sub: std.os.wasi.subscription_t = .{
        .userdata = 0,
        .u = .{
            .tag = .CLOCK,
            .u = .{
                .clock = .{
                    .id = .MONOTONIC,
                    .timeout = ns,
                    .precision = 0,
                    .flags = 0, // not ABSTIME — relative timeout
                },
            },
        },
    };
    var ev: std.os.wasi.event_t = undefined;
    var n_events: usize = 0;
    _ = std.os.wasi.poll_oneoff(&sub, &ev, 1, &n_events);
}

/// ``after MS`` — block for ``ms`` milliseconds, draining any
/// events whose deadlines fall inside the window.  Returns when
/// the deadline is reached (or earlier if the queue empties and no
/// future event is pending).
pub fn sleep_ms(ms: i64) void {
    if (ms <= 0) return;
    const deadline = now_us() + ms * 1_000;
    while (true) {
        if (try_drain_one_timer()) continue;
        if (try_drain_one_idle()) continue;
        const now = now_us();
        if (now >= deadline) return;
        // Sleep until the closer of: deadline, next pending timer.
        const next_deadline = g_timers.peek_deadline();
        const wake = if (next_deadline) |d| @min(d, deadline) else deadline;
        sleep_until(wake);
    }
}

/// ``vwait varName`` — block until the named (global-scoped) var is
/// written, draining events in the meantime.  Returns 0 on success
/// or a negative status:
///   -1 — ``vwait`` nesting overflow (more than 8 deep)
///   -2 — drained empty before the var was written (no events left
///        to wait on; matches Tcl's "would block forever" condition).
pub fn vwait_drain(var_name_ptr: u32, var_name_len: u32) i32 {
    if (!g_vwait.push(var_name_ptr, var_name_len)) return -1;
    defer g_vwait.pop();

    while (!g_vwait.current_done()) {
        // Try to make progress on idle / due timers first.
        if (try_drain_one_timer()) continue;
        if (try_drain_one_idle()) continue;

        // No event ready right now — if the queues are empty AND we
        // have no future timer, we'd block forever.  Bail out.
        const next_deadline = g_timers.peek_deadline();
        if (next_deadline == null and g_idle.empty()) {
            return -2;
        }
        // Sleep up to the next timer's deadline so we don't spin.
        if (next_deadline) |d| sleep_until(d);
    }
    return 0;
}

/// Hook called by the global-variable write path.  Cheap fast-path
/// when no ``vwait`` is active.
pub fn note_var_write(name_ptr: u32, name_len: u32) void {
    g_vwait.note_var_write(name_ptr, name_len);
}

// -- fileevent passthrough ----------------------------------------------------

pub fn fileevent_set_readable(name_ptr: u32, name_len: u32, script_obj: i32) void {
    if (script_obj != 0) tcl_obj_retain(script_obj);
    const prev = g_fileevents.get_readable(name_ptr, name_len);
    if (prev != 0) tcl_obj_release(prev);
    g_fileevents.set_readable(name_ptr, name_len, script_obj);
}

pub fn fileevent_set_writable(name_ptr: u32, name_len: u32, script_obj: i32) void {
    if (script_obj != 0) tcl_obj_retain(script_obj);
    const prev = g_fileevents.get_writable(name_ptr, name_len);
    if (prev != 0) tcl_obj_release(prev);
    g_fileevents.set_writable(name_ptr, name_len, script_obj);
}

pub fn fileevent_get_readable(name_ptr: u32, name_len: u32) i32 {
    return g_fileevents.get_readable(name_ptr, name_len);
}

pub fn fileevent_get_writable(name_ptr: u32, name_len: u32) i32 {
    return g_fileevents.get_writable(name_ptr, name_len);
}
