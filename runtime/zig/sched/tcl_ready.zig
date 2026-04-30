// Idle queue + after-id pool for the WASM scheduler.
//
// The "ready queue" of expired timers is materialised on-the-fly by
// the tick loop draining the timer heap; we don't store it
// separately.  This module owns:
//
//   * the FIFO of ``after idle`` scripts (drained by ``update`` and
//     ``update idletasks``), and
//
//   * the live-id table ``after cancel`` consults so an id known
//     only to the user (not currently in the timer heap because it
//     was a one-shot ms=0 timer that already ran) returns "no such
//     id" rather than silently succeeding.  Real Tcl's contract is
//     "after cancel of an unknown id is a silent no-op", which we
//     match — the table is an internal invariant for ``after info``.
//
// Storage is a singly-linked list anchored in the module's mutable
// state.  Linked-list nodes are bump-allocated alongside the rest of
// the runtime; we never free them.  Idle scripts are short-lived
// (drained on the next ``update``) so the unfreed slack is bounded
// by a single update cycle's worth of registrations.

const obj = @import("../valtypes/tcl_obj.zig");

pub const IdleEntry = struct {
    id: u32,
    script_obj: i32,
    next: u32, // address of next IdleEntry, 0 == nil
};

pub const Queue = struct {
    head: u32 = 0,
    tail: u32 = 0,

    fn node(addr: u32) *IdleEntry {
        return @ptrFromInt(addr);
    }

    pub fn push(self: *Queue, id: u32, script_obj: i32) void {
        const addr = obj.alloc(@sizeOf(IdleEntry));
        const e = node(addr);
        e.id = id;
        e.script_obj = script_obj;
        e.next = 0;
        if (self.tail == 0) {
            self.head = addr;
            self.tail = addr;
        } else {
            node(self.tail).next = addr;
            self.tail = addr;
        }
    }

    pub fn pop(self: *Queue) ?IdleEntry {
        if (self.head == 0) return null;
        const e = node(self.head).*;
        self.head = e.next;
        if (self.head == 0) self.tail = 0;
        return e;
    }

    pub fn empty(self: *const Queue) bool {
        return self.head == 0;
    }

    /// Remove the first entry whose id matches.  Returns the
    /// removed entry's script_obj, or 0 on miss.
    pub fn cancel_by_id(self: *Queue, id: u32) i32 {
        var prev: u32 = 0;
        var cur: u32 = self.head;
        while (cur != 0) {
            const e = node(cur);
            if (e.id == id) {
                if (prev == 0) self.head = e.next else node(prev).next = e.next;
                if (cur == self.tail) self.tail = prev;
                return e.script_obj;
            }
            prev = cur;
            cur = e.next;
        }
        return 0;
    }

    /// Remove the first entry whose script bytes match.  Returns the
    /// cancelled id, or 0 on miss.
    pub fn cancel_by_script(self: *Queue, script_ptr: u32, script_len: u32) u32 {
        var prev: u32 = 0;
        var cur: u32 = self.head;
        while (cur != 0) {
            const e = node(cur);
            const so = obj.obj_ensure_string(e.script_obj);
            const same = blk: {
                if (so.len != script_len) break :blk false;
                const a: [*]const u8 = @ptrFromInt(so.ptr);
                const b: [*]const u8 = @ptrFromInt(script_ptr);
                var k: u32 = 0;
                while (k < script_len) : (k += 1) {
                    if (a[k] != b[k]) break :blk false;
                }
                break :blk true;
            };
            if (same) {
                const removed_id = e.id;
                if (prev == 0) self.head = e.next else node(prev).next = e.next;
                if (cur == self.tail) self.tail = prev;
                return removed_id;
            }
            prev = cur;
            cur = e.next;
        }
        return 0;
    }

    pub fn has_id(self: *const Queue, id: u32) bool {
        var cur: u32 = self.head;
        while (cur != 0) {
            const e = node(cur);
            if (e.id == id) return true;
            cur = e.next;
        }
        return false;
    }

    /// Walk for ``after info``.  ``i`` is a 0-based index in queue
    /// order.  Returns null when out of range.
    pub fn at(self: *const Queue, i: u32) ?*IdleEntry {
        var cur: u32 = self.head;
        var k: u32 = 0;
        while (cur != 0) {
            if (k == i) return node(cur);
            cur = node(cur).next;
            k += 1;
        }
        return null;
    }

    pub fn count(self: *const Queue) u32 {
        var cur: u32 = self.head;
        var n: u32 = 0;
        while (cur != 0) : (cur = node(cur).next) n += 1;
        return n;
    }
};
