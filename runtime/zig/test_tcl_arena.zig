// Unit tests for ``valtypes/tcl_arena.zig`` — the per-statement
// scratch arena (``arena_save`` / ``arena_restore`` / overflow-to-
// libc fallback) introduced in S6.3.
//
// The arena is fertile ground for off-by-one and free-list-walk bugs
// because it routes between two backing strategies (bump-pointer +
// libc fallback) and the round-trip through ``LibcRec`` book-keeping
// is the path most likely to leak under OOM.  These tests pin the
// public contract: save/restore is O(1), allocations are 8-byte
// aligned, and ``arena_restore`` reclaims BOTH arena and libc
// allocations made since the save point.

const std = @import("std");
const testing = std.testing;

const arena = @import("valtypes/tcl_arena.zig");

// ---- alloc bookkeeping ---------------------------------------------

test "arena_alloc_or_libc(0) returns the empty allocation" {
    const saved = arena.arena_save();
    defer arena.arena_restore(saved);

    const a = arena.arena_alloc_or_libc(0);
    try testing.expectEqual(@as(u32, 0), a.addr);
    try testing.expectEqual(@as(u32, 0), a.size);
    try testing.expect(a.from_arena);
    // Cursor untouched.
    try testing.expectEqual(saved.cursor, arena.arena_in_use());
}

test "arena_alloc_or_libc rounds size up to 8-byte alignment" {
    const saved = arena.arena_save();
    defer arena.arena_restore(saved);

    // Request 1 byte — should consume 8.
    const before = arena.arena_in_use();
    const a = arena.arena_alloc_or_libc(1);
    try testing.expect(a.from_arena);
    try testing.expectEqual(@as(u32, 8), a.size);
    try testing.expectEqual(before + 8, arena.arena_in_use());
    // Address itself is 8-byte aligned.
    try testing.expectEqual(@as(u32, 0), a.addr % 8);
}

test "arena_alloc_or_libc successive allocations are contiguous + aligned" {
    const saved = arena.arena_save();
    defer arena.arena_restore(saved);

    const a = arena.arena_alloc_or_libc(7); // → 8
    const b = arena.arena_alloc_or_libc(9); // → 16
    const c = arena.arena_alloc_or_libc(16); // → 16

    // Each allocation lives immediately after the previous one once
    // sizes are aligned up.
    try testing.expectEqual(a.addr + a.size, b.addr);
    try testing.expectEqual(b.addr + b.size, c.addr);
    // All from arena.
    try testing.expect(a.from_arena and b.from_arena and c.from_arena);
}

// ---- save / restore -------------------------------------------------

test "arena_save + arena_restore is O(1) and reclaims bytes" {
    const before = arena.arena_in_use();
    const saved = arena.arena_save();

    _ = arena.arena_alloc_or_libc(4096);
    _ = arena.arena_alloc_or_libc(64);
    _ = arena.arena_alloc_or_libc(8);
    try testing.expect(arena.arena_in_use() > before);

    arena.arena_restore(saved);
    try testing.expectEqual(before, arena.arena_in_use());
}

test "nested save / restore returns each level to its checkpoint" {
    const outer_before = arena.arena_in_use();
    const outer = arena.arena_save();
    _ = arena.arena_alloc_or_libc(64);

    const mid_before = arena.arena_in_use();
    const mid = arena.arena_save();
    _ = arena.arena_alloc_or_libc(64);

    const inner = arena.arena_save();
    _ = arena.arena_alloc_or_libc(64);

    // Inner restore takes us back to the mid checkpoint.
    arena.arena_restore(inner);
    try testing.expectEqual(mid_before + 64, arena.arena_in_use());

    // Mid restore takes us back to outer + first allocation.
    arena.arena_restore(mid);
    try testing.expectEqual(mid_before, arena.arena_in_use());

    // Outer restore goes all the way back.
    arena.arena_restore(outer);
    try testing.expectEqual(outer_before, arena.arena_in_use());
}

// ---- overflow / libc fallback --------------------------------------

test "arena_alloc_or_libc falls back to libc when request > arena" {
    const saved = arena.arena_save();
    defer arena.arena_restore(saved);

    // ``ARENA_SIZE`` is 64 KiB; +1 forces fallback.
    const big: u32 = arena.ARENA_SIZE + 1;
    const a = arena.arena_alloc_or_libc(big);
    try testing.expect(a.addr != 0);
    try testing.expect(!a.from_arena);
    // Address should NOT be inside the arena buffer's aligned span.
    // (We can't trivially compute the buffer base from outside the
    // module, so just assert the round-trip releases cleanly via
    // arena_free without trapping.)
    arena.arena_free(a);
}

test "arena_restore reclaims libc-fallback allocations" {
    const saved = arena.arena_save();
    // Force two libc fallback allocations.
    _ = arena.arena_alloc_or_libc(arena.ARENA_SIZE + 1);
    _ = arena.arena_alloc_or_libc(arena.ARENA_SIZE + 1);

    // No explicit ``arena_free`` — the libc bookkeeping list should
    // be walked by ``arena_restore`` and every entry freed.  If it
    // weren't, this would leak; the test itself can't observe the
    // leak directly but the libc allocator and address-sanitiser
    // builds catch it.  We at least assert restore returns cleanly.
    arena.arena_restore(saved);
    try testing.expectEqual(saved.cursor, arena.arena_in_use());
}

test "arena_free on arena allocation is a no-op (cursor unchanged)" {
    const saved = arena.arena_save();
    defer arena.arena_restore(saved);

    const a = arena.arena_alloc_or_libc(64);
    const before = arena.arena_in_use();
    arena.arena_free(a);
    // Bump pointer doesn't move on free — only ``arena_restore``
    // reclaims arena bytes.
    try testing.expectEqual(before, arena.arena_in_use());
}

// ---- diagnostic counters -------------------------------------------

test "arena_in_use + arena_remaining sum to ARENA_SIZE" {
    const saved = arena.arena_save();
    defer arena.arena_restore(saved);

    _ = arena.arena_alloc_or_libc(128);
    try testing.expectEqual(arena.ARENA_SIZE, arena.arena_in_use() + arena.arena_remaining());
}
