// Min-heap of pending ``after MS`` timers.
//
// 4-ary min-heap keyed on absolute monotonic-µs deadline.  4-ary
// (vs binary) gives a shallower tree and better cache locality for
// the realistic working-set size of 10s–100s of pending timers.
// Each entry holds:
//
//     id          : u32   monotonically-increasing per-scheduler id
//     deadline_us : i64   absolute monotonic µs (compared to the
//                         scheduler's clock-µs reading)
//     script_obj  : i32   TclObj handle for the body to evaluate
//                         when the deadline expires.  Retained while
//                         the entry sits in the heap so the body
//                         survives the caller's ``after`` invocation.
//
// The heap is a flat array; capacity grows on insert via realloc.
// Cancellation is by id: a linear scan finds the slot, the last
// element fills it, and the displaced element sifts up or down.
//
// Linear scan is acceptable because the timer count is small and the
// scan is cache-friendly (8 bytes per entry on the hot field).  If
// we ever ship a workload with thousands of pending timers, a side
// hash from id → slot is a 5-line follow-up.

const std = @import("std");
const obj = @import("../valtypes/tcl_obj.zig");

pub const TimerEntry = struct {
    id: u32,
    deadline_us: i64,
    script_obj: i32,
    /// 0 = ``after MS script``; 1 = ``after idle script`` (these are
    /// not stored in the timer heap — see :mod:`tcl_ready` — but we
    /// reserve the field so callers see a stable ``after info`` shape).
    is_idle: u8,
};

const INITIAL_CAP: u32 = 16;

pub const Heap = struct {
    base: u32 = 0, // address of TimerEntry[]
    len: u32 = 0,
    cap: u32 = 0,

    pub fn ensure_cap(self: *Heap, want: u32) void {
        if (self.cap >= want) return;
        var new_cap: u32 = if (self.cap == 0) INITIAL_CAP else self.cap;
        while (new_cap < want) new_cap *= 2;
        const new_base = obj.alloc(new_cap * @sizeOf(TimerEntry));
        if (self.base != 0 and self.len > 0) {
            obj.memcpy(new_base, self.base, self.len * @sizeOf(TimerEntry));
        }
        // Old buffer leaks here — the runtime's bump allocator
        // doesn't reclaim, and the heap grows monotonically up to
        // its working-set ceiling.  Acceptable for the timer heap.
        self.base = new_base;
        self.cap = new_cap;
    }

    fn slot(self: *const Heap, i: u32) *TimerEntry {
        return @ptrFromInt(self.base + i * @sizeOf(TimerEntry));
    }

    fn less(a: *const TimerEntry, b: *const TimerEntry) bool {
        return a.deadline_us < b.deadline_us;
    }

    fn swap(self: *Heap, i: u32, j: u32) void {
        const a = self.slot(i).*;
        const b = self.slot(j).*;
        self.slot(i).* = b;
        self.slot(j).* = a;
    }

    fn sift_up(self: *Heap, start: u32) void {
        var i = start;
        while (i > 0) {
            const parent = (i - 1) / 4;
            if (less(self.slot(i), self.slot(parent))) {
                self.swap(i, parent);
                i = parent;
            } else break;
        }
    }

    fn sift_down(self: *Heap, start: u32) void {
        var i = start;
        while (true) {
            const first_child = i * 4 + 1;
            if (first_child >= self.len) break;
            var best = i;
            var c = first_child;
            const last = @min(first_child + 4, self.len);
            while (c < last) : (c += 1) {
                if (less(self.slot(c), self.slot(best))) best = c;
            }
            if (best == i) break;
            self.swap(i, best);
            i = best;
        }
    }

    pub fn push(self: *Heap, entry: TimerEntry) void {
        self.ensure_cap(self.len + 1);
        self.slot(self.len).* = entry;
        const idx = self.len;
        self.len += 1;
        self.sift_up(idx);
    }

    /// Peek the next-to-fire deadline, or null if empty.
    pub fn peek_deadline(self: *const Heap) ?i64 {
        if (self.len == 0) return null;
        return self.slot(0).deadline_us;
    }

    /// Pop the earliest entry if its deadline ≤ now_us.  Returns null
    /// otherwise.  Caller is responsible for releasing ``script_obj``
    /// after dispatching the script.
    pub fn pop_if_due(self: *Heap, now_us: i64) ?TimerEntry {
        if (self.len == 0) return null;
        if (self.slot(0).deadline_us > now_us) return null;
        const top = self.slot(0).*;
        self.len -= 1;
        if (self.len > 0) {
            self.slot(0).* = self.slot(self.len).*;
            self.sift_down(0);
        }
        return top;
    }

    /// Remove the entry whose id matches.  Returns the script_obj
    /// (so the caller can release it) or 0 on miss.
    pub fn cancel_by_id(self: *Heap, id: u32) i32 {
        var i: u32 = 0;
        while (i < self.len) : (i += 1) {
            if (self.slot(i).id == id) {
                const removed_script = self.slot(i).script_obj;
                self.len -= 1;
                if (i != self.len) {
                    self.slot(i).* = self.slot(self.len).*;
                    // The displaced element may need to move either way.
                    self.sift_down(i);
                    self.sift_up(i);
                }
                return removed_script;
            }
        }
        return 0;
    }

    /// Result of cancel-by-script: the removed entry's id (0 on miss)
    /// plus its retained script TclObj so the caller can release it.
    pub const CancelResult = struct { id: u32, script_obj: i32 };

    /// Remove the first entry whose script bytes equal ``s``.  Returns
    /// ``{id, script_obj}`` for the removed entry or ``{0, 0}`` on miss.
    /// Caller is responsible for releasing ``script_obj`` — it was
    /// retained at schedule time.
    pub fn cancel_by_script(self: *Heap, script_ptr: u32, script_len: u32) CancelResult {
        var i: u32 = 0;
        while (i < self.len) : (i += 1) {
            const e = self.slot(i);
            const so = obj.obj_ensure_string(e.script_obj);
            if (so.len != script_len) continue;
            const a: [*]const u8 = @ptrFromInt(so.ptr);
            const b: [*]const u8 = @ptrFromInt(script_ptr);
            var match = true;
            var k: u32 = 0;
            while (k < script_len) : (k += 1) {
                if (a[k] != b[k]) {
                    match = false;
                    break;
                }
            }
            if (match) {
                const id = e.id;
                const script = e.script_obj;
                self.len -= 1;
                if (i != self.len) {
                    self.slot(i).* = self.slot(self.len).*;
                    self.sift_down(i);
                    self.sift_up(i);
                }
                return .{ .id = id, .script_obj = script };
            }
        }
        return .{ .id = 0, .script_obj = 0 };
    }

    /// Linear scan: return true if ``id`` is in the heap.
    pub fn has_id(self: *const Heap, id: u32) bool {
        var i: u32 = 0;
        while (i < self.len) : (i += 1) {
            if (self.slot(i).id == id) return true;
        }
        return false;
    }

    /// Walk every pending timer in heap order (NOT deadline order).
    /// Used by ``after info`` to enumerate ids.
    pub fn at(self: *const Heap, i: u32) ?*TimerEntry {
        if (i >= self.len) return null;
        return self.slot(i);
    }
};
