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

/// Save the current arena cursor.  Pair with ``arena_restore``
/// at the end of the scratch scope.
pub fn arena_save() u32 {
    return arena_cursor;
}

/// Restore the arena cursor to ``saved``, releasing every arena
/// allocation made since the matching ``arena_save``.  Libc
/// fallback allocations made in the same scope are not affected
/// — those need their own ``arena_free`` calls.
pub fn arena_restore(saved: u32) void {
    arena_cursor = saved;
}

/// Allocate ``size`` bytes from the arena, with libc fallback.
///
/// The returned ``Allocation`` records the address and whether
/// the allocation came from the arena.  Pass it to ``arena_free``
/// (or just rely on ``arena_restore`` to drop the arena bytes
/// in bulk).
pub fn arena_alloc_or_libc(size: u32) Allocation {
    if (size == 0) return .{ .addr = 0, .size = 0, .from_arena = true };
    const aligned: u32 = (size + 7) & ~@as(u32, 7);
    if (arena_cursor + aligned <= ARENA_SIZE) {
        const base: u32 = @intCast(@intFromPtr(&arena_buffer));
        const addr = base + arena_cursor;
        arena_cursor += aligned;
        return .{ .addr = addr, .size = aligned, .from_arena = true };
    }
    // Arena is full — fall back to libc.  The caller's
    // ``arena_free`` handles the cleanup.
    const addr = obj.alloc(aligned);
    return .{ .addr = addr, .size = aligned, .from_arena = false };
}

/// Free an allocation returned by ``arena_alloc_or_libc``.
///
/// For arena allocations this is a no-op — the matching
/// ``arena_restore`` reclaims them in bulk.  For libc-fallback
/// allocations this routes to ``free_sized``.  Calling
/// ``arena_free`` even on arena allocations is safe and lets
/// callers use the same cleanup path regardless of where the
/// allocation came from.
pub fn arena_free(a: Allocation) void {
    if (a.from_arena) return;
    if (a.addr == 0) return;
    obj.free_sized(a.addr, a.size);
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
