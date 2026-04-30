// S6.3 — Per-statement arena for parser / subst scratch.
//
// Many runtime helpers allocate short-lived byte buffers whose
// lifetime ends before the helper returns:
//
//   * ``subst_flagged`` allocates two scratch arrays (the "pieces"
//     buffer recording (ptr, len) for each substitution, and a
//     parallel array of retained TclObj handles) and frees them
//     both before returning the concatenated result.
//
//   * Future paths (regex match buffers, parse-token vectors)
//     have the same lifetime pattern.
//
// Routing those allocations through ``alloc()`` / ``free_sized()``
// pays the libc-malloc round-trip cost on every statement.  This
// arena replaces both with a single bump-pointer allocation and a
// pointer-reset on scope exit.  No ``free`` calls happen at all
// for the common case where the arena has room.
//
// **Overflow fallback.**  The arena is fixed-size (``ARENA_SIZE``).
// If a request doesn't fit, ``arena_alloc_or_libc`` falls back to
// ``alloc()`` and tags the returned :class:`Allocation` so
// ``arena_free`` knows to ``free_sized`` instead of relying on
// arena reset.  Mixing both within a single scope is fine.
//
// **Save / restore pattern.**  Callers ``arena_save()`` at the
// top of their scratch scope and ``arena_restore(saved)`` at the
// bottom.  All arena allocations made between those calls are
// reclaimed in O(1).  Nested scopes (e.g. ``subst`` calling
// ``eval_command`` calling ``subst`` again) work transparently
// because each frame restores its own cursor — the inner scope's
// allocations are released by the inner restore, and the outer
// frame's cursor is already past the inner allocations so its
// restore is a no-op for them.
//
// **Libc-fallback bulk cleanup (PR #237 review).**  Earlier
// versions of ``arena_restore`` reset only the bump cursor — when
// the arena saturated and a few allocations went to libc, those
// libc buffers persisted past the scope's end and leaked under
// any long-running workload that hit saturation more than once.
// We track libc-fallback allocations in a singly-linked list
// keyed off ``LibcRec`` and walk-and-free everything appended
// since the matching ``arena_save``.  ``arena_save`` snapshots
// the list head; ``arena_restore`` frees every entry strictly
// above the snapshot.  Cost per fallback alloc: one list-head
// update + one 12-byte ``LibcRec`` from libc itself.
//
// **Soundness.**  The arena's bytes must NOT outlive the scope
// that allocated them.  Anything that escapes (e.g. a buffer
// stored as a TclObj's heap-owned ``str_ptr``) must come from
// ``alloc()`` directly, not from this arena.

const std = @import("std");
const obj = @import("tcl_obj.zig");

pub const ARENA_SIZE: u32 = 65536; // 64 KiB

/// The arena buffer itself — a single contiguous region in
/// linear memory backing every scratch allocation.
var arena_buffer: [ARENA_SIZE]u8 align(8) = [_]u8{0} ** ARENA_SIZE;

/// Bytes used so far.  Always 8-byte aligned after each alloc so
/// successive bumps stay aligned.
var arena_cursor: u32 = 0;

/// Tagged allocation: ``addr`` plus a flag telling the matching
/// ``arena_free`` whether to reset (arena) or call ``free_sized``
/// (libc fallback).
pub const Allocation = struct {
    addr: u32,
    size: u32,
    from_arena: bool,
};

/// Bookkeeping record for a single libc-fallback allocation.
/// Stored in a singly-linked list rooted at ``libc_head`` so
/// ``arena_restore`` can free every libc allocation made since
/// the matching ``arena_save``.  The record itself is allocated
/// from libc (small fixed size, freed in lockstep with the buffer
/// it tracks).
const LibcRec = extern struct {
    addr: u32,
    size: u32,
    next: u32,
};

const LIBC_REC_SIZE: u32 = @sizeOf(LibcRec);

var libc_head: u32 = 0;

/// Composite save token — captures both the arena cursor and the
/// libc-fallback list head so ``arena_restore`` can roll back
/// both axes atomically.
pub const Saved = struct {
    cursor: u32,
    libc_head: u32,
};

/// Save the current arena cursor and libc-fallback list head.
/// Pair with ``arena_restore`` at the end of the scratch scope.
pub fn arena_save() Saved {
    return .{ .cursor = arena_cursor, .libc_head = libc_head };
}

/// Restore the arena cursor to ``saved``, releasing every arena
/// allocation made since the matching ``arena_save``.  Also walks
/// the libc-fallback list and frees every entry appended after
/// the saved snapshot — fixes the leak the previous version had
/// when the arena saturated and overflow allocations went to
/// libc (PR #237 review).
pub fn arena_restore(saved: Saved) void {
    while (libc_head != saved.libc_head) {
        const rec_ptr = libc_head;
        const rec: *const LibcRec = @ptrFromInt(rec_ptr);
        const next = rec.next;
        const addr = rec.addr;
        const size = rec.size;
        if (addr != 0) obj.free_sized(addr, size);
        obj.free_sized(rec_ptr, LIBC_REC_SIZE);
        libc_head = next;
    }
    arena_cursor = saved.cursor;
}

/// Allocate ``size`` bytes from the arena, with libc fallback.
///
/// The returned ``Allocation`` records the address and whether
/// the allocation came from the arena.  Pass it to ``arena_free``
/// (or just rely on ``arena_restore`` to drop both arena and
/// libc-fallback bytes in bulk).
pub fn arena_alloc_or_libc(size: u32) Allocation {
    if (size == 0) return .{ .addr = 0, .size = 0, .from_arena = true };
    const aligned: u32 = (size + 7) & ~@as(u32, 7);
    if (arena_cursor + aligned <= ARENA_SIZE) {
        const base: u32 = @intCast(@intFromPtr(&arena_buffer));
        const addr = base + arena_cursor;
        arena_cursor += aligned;
        return .{ .addr = addr, .size = aligned, .from_arena = true };
    }
    // Arena is full — fall back to libc and register the allocation
    // on the libc-fallback list so ``arena_restore`` can free it
    // even if the caller doesn't explicitly call ``arena_free``.
    const addr = obj.alloc(aligned);
    if (addr == 0) {
        return .{ .addr = 0, .size = aligned, .from_arena = false };
    }
    const rec_addr = obj.alloc(LIBC_REC_SIZE);
    if (rec_addr != 0) {
        const rec: *LibcRec = @ptrFromInt(rec_addr);
        rec.addr = addr;
        rec.size = aligned;
        rec.next = libc_head;
        libc_head = rec_addr;
    }
    return .{ .addr = addr, .size = aligned, .from_arena = false };
}

/// Free an allocation returned by ``arena_alloc_or_libc``.
///
/// For arena allocations this is a no-op — the matching
/// ``arena_restore`` reclaims them in bulk.  For libc-fallback
/// allocations this routes to ``free_sized`` and unlinks the
/// matching ``LibcRec`` from the bookkeeping list (so
/// ``arena_restore`` doesn't double-free).  Calling
/// ``arena_free`` even on arena allocations is safe and lets
/// callers use the same cleanup path regardless of where the
/// allocation came from.
pub fn arena_free(a: Allocation) void {
    if (a.from_arena) return;
    if (a.addr == 0) return;
    obj.free_sized(a.addr, a.size);
    var prev: u32 = 0;
    var cur: u32 = libc_head;
    while (cur != 0) {
        const rec: *const LibcRec = @ptrFromInt(cur);
        if (rec.addr == a.addr) {
            const next = rec.next;
            if (prev == 0) {
                libc_head = next;
            } else {
                const prev_rec: *LibcRec = @ptrFromInt(prev);
                prev_rec.next = next;
            }
            obj.free_sized(cur, LIBC_REC_SIZE);
            return;
        }
        prev = cur;
        cur = rec.next;
    }
}

/// Diagnostic counter — number of bytes currently in use in the
/// arena.  Exposed for tests; production code shouldn't depend
/// on the exact value (it's reset by every scope).
pub fn arena_in_use() u32 {
    return arena_cursor;
}

/// Diagnostic counter — bytes available before the arena would
/// fall back to libc.
pub fn arena_remaining() u32 {
    return ARENA_SIZE - arena_cursor;
}
