// Unit tests for ``valtypes/hash_table.zig`` — open-addressed string-
// keyed hash primitive shared by the proc / globals / per-frame
// tables.  Ports of upstream Tcl rely on its probe-chain semantics
// being correct; in particular tombstone reuse and grow-rehash are
// historically rich sources of bugs (``Tcl_HashTable``'s rebuild loop
// in ``generic/tclHash.c`` has been rewritten more than once over
// the years).
//
// Wave 2 of the WASM-runtime test harness — see
// ``runtime/zig/build.zig``'s ``test`` step.

const std = @import("std");
const testing = std.testing;

const ht = @import("valtypes/hash_table.zig");
const obj = @import("valtypes/tcl_obj.zig");

// ---- helpers --------------------------------------------------------

/// 16-byte buckets (12 byte header + 4 byte value payload).  Matches
/// the smallest real consumer (the global namespace's command table
/// stores a single command-handle ``i32`` per entry).
const Table4 = ht.Table(16);

fn ptrOf(s: []const u8) u32 {
    return @intCast(@intFromPtr(s.ptr));
}

fn lenOf(s: []const u8) u32 {
    return @intCast(s.len);
}

fn hashOf(s: []const u8) u32 {
    return ht.fnv1a(ptrOf(s), lenOf(s));
}

fn lookup(t: *const Table4, key: []const u8) ?u32 {
    return t.find(ptrOf(key), lenOf(key), hashOf(key));
}

fn put(t: *Table4, key: []const u8, value: i32) u32 {
    const base = t.insert_header(ptrOf(key), lenOf(key), hashOf(key));
    obj.write_i32(base + ht.HEADER_SIZE, value);
    return base;
}

// ---- fnv1a ----------------------------------------------------------

test "fnv1a — basis for empty input, deterministic for ASCII" {
    // FNV-1a 32-bit basis.
    try testing.expectEqual(@as(u32, 2166136261), ht.fnv1a(0, 0));
    try testing.expectEqual(@as(u32, 2166136261), ht.fnv1a(1234, 0));

    // Same bytes hash to the same value across calls.
    try testing.expectEqual(hashOf("hello"), hashOf("hello"));
    // Different bytes to different values.
    try testing.expect(hashOf("hello") != hashOf("world"));
    // One-bit difference still produces avalanche.
    try testing.expect(hashOf("Hello") != hashOf("hello"));
}

test "fnv1a — known FNV-1a 32-bit reference vectors" {
    // Reference values from
    // http://www.isthe.com/chongo/tech/comp/fnv/index.html#FNV-test-vectors
    try testing.expectEqual(@as(u32, 0xe40c292c), hashOf("a"));
    try testing.expectEqual(@as(u32, 0xbf9cf968), hashOf("foobar"));
}

// ---- init / find on empty -------------------------------------------

test "Table.init allocates and zero-initialises" {
    var t: Table4 = .{};
    t.init(8);
    try testing.expectEqual(@as(u32, 8), t.cap);
    try testing.expectEqual(@as(u32, 0), t.count);
    try testing.expect(t.buf != 0);
    try testing.expect(t.nonzero());
}

test "Table.init is idempotent" {
    var t: Table4 = .{};
    t.init(8);
    const first_buf = t.buf;
    t.init(64); // second call should be a no-op
    try testing.expectEqual(first_buf, t.buf);
    try testing.expectEqual(@as(u32, 8), t.cap);
}

test "find on uninitialised table returns null" {
    var t: Table4 = .{};
    try testing.expect(lookup(&t, "anything") == null);
    try testing.expect(!t.nonzero());
}

test "find on empty initialised table returns null" {
    var t: Table4 = .{};
    t.init(8);
    try testing.expect(lookup(&t, "missing") == null);
}

// ---- basic insert / find --------------------------------------------

test "insert_header stores key + payload and find recovers them" {
    var t: Table4 = .{};
    t.init(8);
    const base = put(&t, "hello", 0xCAFE);
    try testing.expectEqual(@as(u32, 1), t.count);

    const found = lookup(&t, "hello").?;
    try testing.expectEqual(base, found);
    try testing.expectEqual(@as(i32, 0xCAFE), obj.read_i32(found + ht.HEADER_SIZE));
}

test "re-inserting the same key returns the existing bucket (no dup)" {
    var t: Table4 = .{};
    t.init(8);
    const first = put(&t, "key", 1);
    // ``try_insert_header`` is the path that detects an existing
    // live entry on the chain; ``insert_header`` falls back to it.
    const second = t.try_insert_header(ptrOf("key"), lenOf("key"), hashOf("key")).?;
    try testing.expectEqual(first, second);
    try testing.expectEqual(@as(u32, 1), t.count);
}

test "many distinct keys all roundtrip through find" {
    var t: Table4 = .{};
    t.init(64);
    const keys = [_][]const u8{
        "alpha", "beta", "gamma", "delta", "epsilon",
        "zeta",  "eta",  "theta", "iota",  "kappa",
    };
    for (keys, 0..) |k, i| {
        _ = put(&t, k, @intCast(i));
    }
    try testing.expectEqual(@as(u32, keys.len), t.count);
    for (keys, 0..) |k, i| {
        const base = lookup(&t, k).?;
        try testing.expectEqual(@as(i32, @intCast(i)), obj.read_i32(base + ht.HEADER_SIZE));
    }
}

// ---- needs_grow / grow ---------------------------------------------

test "needs_grow fires at 75% load factor" {
    var t: Table4 = .{};
    t.init(8);
    // 8 buckets * 0.75 = 6 — first 5 inserts should not trip.
    var i: u32 = 0;
    var buf: [8]u8 = undefined;
    while (i < 5) : (i += 1) {
        buf[0] = 'k';
        buf[1] = '0' + @as(u8, @intCast(i));
        _ = t.insert_header(@intFromPtr(&buf), 2, ht.fnv1a(@intFromPtr(&buf), 2));
        try testing.expect(!t.needs_grow());
    }
    // 6th insert pushes count to 6, hitting exactly 75% — needs_grow
    // returns true so the caller knows to rehash before continuing.
    buf[0] = 'k';
    buf[1] = '5';
    _ = t.insert_header(@intFromPtr(&buf), 2, ht.fnv1a(@intFromPtr(&buf), 2));
    try testing.expect(t.needs_grow());
}

test "grow doubles capacity and preserves every entry" {
    var t: Table4 = .{};
    t.init(8);
    const keys = [_][]const u8{ "a", "bb", "ccc", "dddd", "eeeee" };
    for (keys, 0..) |k, i| _ = put(&t, k, @intCast(i + 100));

    t.grow();
    try testing.expectEqual(@as(u32, 16), t.cap);
    try testing.expectEqual(@as(u32, keys.len), t.count);

    // Every key still findable, payload preserved.
    for (keys, 0..) |k, i| {
        const base = lookup(&t, k).?;
        try testing.expectEqual(@as(i32, @intCast(i + 100)), obj.read_i32(base + ht.HEADER_SIZE));
    }
}

// ---- delete + tombstone semantics ----------------------------------

test "delete_at removes the entry and find returns null" {
    var t: Table4 = .{};
    t.init(8);
    const base = put(&t, "doomed", 99);
    try testing.expect(lookup(&t, "doomed") != null);

    t.delete_at(base);
    try testing.expectEqual(@as(u32, 0), t.count);
    try testing.expect(lookup(&t, "doomed") == null);
}

test "delete + reinsert reuses the tombstone slot" {
    var t: Table4 = .{};
    t.init(8);
    const before = put(&t, "key", 1);
    t.delete_at(before);
    const after = put(&t, "key", 2);

    // Tombstone reuse: the new entry should land in the same bucket.
    try testing.expectEqual(before, after);
    try testing.expectEqual(@as(u32, 1), t.count);
    try testing.expectEqual(@as(i32, 2), obj.read_i32(after + ht.HEADER_SIZE));
}

test "delete in middle of probe chain doesn't sever later entries" {
    // Force a collision: pick a small table and look for two keys
    // whose FNV-1a hashes share the same probe slot mod cap.  We
    // simulate the worst case by inserting many keys, deleting one
    // mid-chain, and asserting later inserts still resolve.
    var t: Table4 = .{};
    t.init(8);
    const a = put(&t, "alpha", 1);
    _ = put(&t, "beta", 2);
    _ = put(&t, "gamma", 3);
    _ = put(&t, "delta", 4);

    t.delete_at(a);

    // ``alpha`` is now a tombstone; the other keys must still be
    // discoverable — open-addressed delete WITHOUT tombstones would
    // sever any probe chain that hashed past ``alpha``'s slot.
    try testing.expect(lookup(&t, "beta") != null);
    try testing.expect(lookup(&t, "gamma") != null);
    try testing.expect(lookup(&t, "delta") != null);
    try testing.expect(lookup(&t, "alpha") == null);
}

// ---- each / iteration ----------------------------------------------

const Counter = struct {
    n: *u32,
    fn add(self: Counter, _: u32) void {
        self.n.* += 1;
    }
};

test "each visits every populated bucket exactly once" {
    var t: Table4 = .{};
    t.init(8);
    _ = put(&t, "a", 1);
    _ = put(&t, "b", 2);
    _ = put(&t, "c", 3);

    var n: u32 = 0;
    t.each(Counter{ .n = &n }, Counter.add);
    try testing.expectEqual(@as(u32, 3), n);
}

test "each skips tombstones" {
    var t: Table4 = .{};
    t.init(8);
    _ = put(&t, "a", 1);
    const b = put(&t, "b", 2);
    _ = put(&t, "c", 3);
    t.delete_at(b);

    var n: u32 = 0;
    t.each(Counter{ .n = &n }, Counter.add);
    try testing.expectEqual(@as(u32, 2), n);
}

test "each on empty table doesn't fire visitor" {
    var t: Table4 = .{};
    t.init(8);
    var n: u32 = 0;
    t.each(Counter{ .n = &n }, Counter.add);
    try testing.expectEqual(@as(u32, 0), n);
}
