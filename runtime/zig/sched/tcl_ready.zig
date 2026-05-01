// Idle queue for the WASM scheduler.
//
// The "ready queue" of expired timers is materialised on-the-fly by
// the tick loop draining the timer heap; we don't store it
// separately.  This module owns the FIFO of ``after idle`` scripts
// (drained by ``update`` and ``update idletasks``) plus the
// id/script-bytes lookups ``after cancel`` and ``after info``
// consult.  It does NOT keep a separate "live-id" table for
// already-fired one-shots: ``after cancel`` of an unknown id is a
// silent no-op in real Tcl, which is what we get for free by
// returning 0 when the queue (and the timer heap) miss.
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

    /// Result of cancel-by-script: the removed entry's id (0 on miss)
    /// plus its retained script TclObj so the caller can release it.
    pub const CancelResult = struct { id: u32, script_obj: i32 };

    /// Remove the first entry whose script bytes match.  Returns
    /// ``{id, script_obj}`` for the removed entry or ``{0, 0}`` on
    /// miss.  Caller is responsible for releasing ``script_obj`` —
    /// it was retained at schedule time.
    pub fn cancel_by_script(self: *Queue, script_ptr: u32, script_len: u32) CancelResult {
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
                const script = e.script_obj;
                if (prev == 0) self.head = e.next else node(prev).next = e.next;
                if (cur == self.tail) self.tail = prev;
                return .{ .id = removed_id, .script_obj = script };
            }
            prev = cur;
            cur = e.next;
        }
        return .{ .id = 0, .script_obj = 0 };
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
