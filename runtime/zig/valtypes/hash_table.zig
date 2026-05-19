// Shared open-addressed string-keyed hash table primitive.
//
// All three pre-existing tables in this runtime — proc registry
// (``tcl_procs.zig``), global variable table (``tcl_globals.zig``),
// and per-frame local table (``tcl_frames.zig``) — used the same
// 12-byte header (``name_ptr | name_len | hash``) followed by a
// caller-defined value payload, with the same FNV-1a hashing and
// the same linear-probe layout.  This module factors that out so:
//
// * The namespace tree (P1+) can use the same primitive for child /
//   command / variable tables without re-implementing probe logic.
// * Bug fixes to grow / find apply uniformly across every table.
// * The bucket layout becomes a single source of truth: the header
//   is fixed; the trailing payload is whatever the caller wants.
//
// We deliberately do NOT use ``std.HashMap``: our bump allocator
// (``tcl_obj.zig:alloc``) lacks the ``free`` symmetry stdlib maps
// expect, and pulling stdlib hash machinery into this WASM module
// would add ~40-80 KB of bytecode for no measured win.

const obj = @import("tcl_obj.zig");
const alloc = obj.alloc;
const memcpy = obj.memcpy;
const read_i32 = obj.read_i32;
const write_i32 = obj.write_i32;

/// FNV-1a hash over a (ptr, len) byte range.  Empty ranges return
/// the FNV-1a basis directly without dereferencing the pointer —
/// safe for the ``ptr=0, len=0`` representation used by empty
/// strings.
pub fn fnv1a(ptr: u32, len: u32) u32 {
    if (len == 0) return 2166136261;
    var h: u32 = 2166136261;
    const src: [*]const u8 = @ptrFromInt(ptr);
    for (0..len) |i| {
        h ^= @as(u32, src[i]);
        h *%= 16777619;
    }
    return h;
}

/// Bucket header byte size: ``name_ptr (u32) | name_len (u32) |
/// hash (u32)``.  Caller's value payload starts at ``HEADER_SIZE``.
pub const HEADER_SIZE: u32 = 12;

/// Sentinel ``name_ptr`` value marking a deleted bucket.  Picked
/// as ``1`` because the WASM heap base is far above 1 (the data
/// segment occupies the low addresses and ``alloc`` returns
/// pointers above it), so a real key buffer can never have this
/// address.  ``find`` skips tombstones (continues probing rather
/// than returning null), and ``try_insert_header`` reuses the
/// first tombstone encountered.  This lets ``delete``-style
/// callers (currently only ``parse_cache.invalidate_for_buffer``)
/// remove individual entries without breaking probe chains —
/// without tombstones, zeroing a bucket would terminate every
/// probe walk that hashed past it, hiding still-live entries.
pub const TOMBSTONE: u32 = 1;

/// Open-addressed string-keyed table with a comptime-fixed bucket
/// size.  Storage lives in WASM linear memory (``alloc``-backed)
/// and is never freed — buckets are reused on grow.
///
/// Layout per bucket:
///   [0..3]   name_ptr  : u32 — heap copy of the key bytes (0 = empty slot)
///   [4..7]   name_len  : u32
///   [8..11]  hash      : u32 (FNV-1a)
///   [12..]   value     : caller-defined, ``bucket_size - 12`` bytes
///
/// Callers extract / write value bytes at ``base + HEADER_SIZE +
/// offset``; the table never inspects the payload.
pub fn Table(comptime bucket_size: u32) type {
    if (bucket_size < HEADER_SIZE) @compileError("bucket_size must be >= HEADER_SIZE");
    if (bucket_size % 4 != 0) @compileError("bucket_size must be 4-byte aligned");
    return extern struct {
        const Self = @This();

        // ``extern`` so the type is ABI-compatible and can be embedded
        // inside another ``extern struct`` (e.g. ``tcl_ns.Namespace``
        // holds three of these inline).  All fields are u32, so the
        // resulting layout is identical to the prior non-extern
        // variant — this is purely a Zig-language constraint, not a
        // bytewise change.
        buf: u32 = 0,
        cap: u32 = 0,
        count: u32 = 0,

        /// One-time allocation of the initial backing buffer.  Idempotent:
        /// a second call with ``buf != 0`` is a no-op.
        pub fn init(self: *Self, initial_cap: u32) void {
            if (self.buf != 0) return;
            self.cap = initial_cap;
            self.buf = alloc(self.cap * bucket_size);
            var i: u32 = 0;
            while (i < self.cap) : (i += 1) {
                write_i32(self.buf + i * bucket_size, 0); // empty marker
            }
        }

        /// Cheap "any procs registered" check used on the dispatch fast
        /// path before doing real work.  Inlined; the WASM produced
        /// is one ``i32.load`` + branch.
        pub fn nonzero(self: *const Self) bool {
            return self.buf != 0;
        }

        /// Look up a name; returns the bucket's base address (not just
        /// the value) so the caller can read its payload at fixed
        /// offsets without re-hashing.  ``null`` means "not present".
        ///
        /// Tombstones (``name_ptr == TOMBSTONE``) are skipped — probing
        /// continues past them so deletes don't sever the probe chain
        /// for entries that hashed earlier in the chain.
        pub fn find(self: *const Self, name_ptr: u32, name_len: u32, hash: u32) ?u32 {
            if (self.buf == 0) return null;
            const mask = self.cap - 1;
            var idx = hash & mask;
            var probes: u32 = 0;
            while (probes < self.cap) : (probes += 1) {
                const base = self.buf + idx * bucket_size;
                const ep: u32 = @bitCast(read_i32(base));
                if (ep == 0) return null;
                if (ep != TOMBSTONE) {
                    const el: u32 = @bitCast(read_i32(base + 4));
                    const eh: u32 = @bitCast(read_i32(base + 8));
                    if (eh == hash and el == name_len) {
                        // ``@ptrFromInt(0)`` panics under Zig's
                        // debug-build pointer-null check even when
                        // the dereference loop wouldn't run.  Empty-
                        // string keys (``name_len == 0``) match any
                        // other empty key without inspecting bytes,
                        // so short-circuit here.
                        if (el == 0) return base;
                        const sp: [*]const u8 = @ptrFromInt(ep);
                        const np: [*]const u8 = @ptrFromInt(name_ptr);
                        var match = true;
                        for (0..el) |k| {
                            if (sp[k] != np[k]) {
                                match = false;
                                break;
                            }
                        }
                        if (match) return base;
                    }
                }
                idx = (idx + 1) & mask;
            }
            return null;
        }

        /// Insert a new entry: walks the probe chain to an empty slot,
        /// allocates a heap copy of the name bytes, writes the header,
        /// and returns the bucket base.  Caller is responsible for
        /// writing value bytes at ``base + HEADER_SIZE + …``.  Caller
        /// is also responsible for calling ``grow`` first if
        /// ``needs_grow()`` returns true — this method does not auto-
        /// grow because grow may invalidate caches kept on the caller
        /// side (e.g. proc-lookup LRU pointing at bucket bases).
        pub fn insert_header(self: *Self, name_ptr: u32, name_len: u32, hash: u32) u32 {
            return self.try_insert_header(name_ptr, name_len, hash).?;
        }

        /// Like ``insert_header`` but returns ``null`` if the probe chain
        /// makes a full pass without finding an empty slot.  Used by
        /// fixed-capacity tables (e.g. per-frame local tables) that
        /// trap rather than grow when they overflow — they need to
        /// detect "full" cleanly instead of looping forever.
        ///
        /// Reuses the first tombstone (``name_ptr == TOMBSTONE``) seen
        /// in the probe chain, BUT only after confirming the key isn't
        /// already present further down the chain — otherwise a
        /// re-insert of a still-live key would land in the tombstone
        /// slot while the live entry stays intact, leading to two
        /// buckets for the same key.
        pub fn try_insert_header(self: *Self, name_ptr: u32, name_len: u32, hash: u32) ?u32 {
            const mask = self.cap - 1;
            var idx = hash & mask;
            var probes: u32 = 0;
            var first_tombstone: u32 = 0; // 0 = none seen
            while (probes < self.cap) : (probes += 1) {
                const base = self.buf + idx * bucket_size;
                const ep: u32 = @bitCast(read_i32(base));
                if (ep == 0) {
                    // End of probe chain — fall through to insert.
                    const target = if (first_tombstone != 0) first_tombstone else base;
                    const nbuf = alloc(name_len);
                    // ``alloc`` returns 0 on OOM (and raises ``oom_flag``).
                    // Surface as ``null`` so the caller's existing
                    // "table full" handling path covers OOM too — without
                    // this guard, ``memcpy`` would call ``@ptrFromInt(0)``
                    // and trip the debug "cast causes pointer to be null"
                    // panic.  Leaves the bucket untouched so a retry after
                    // the OOM clears can still place the entry.
                    if (nbuf == 0 and name_len != 0) return null;
                    memcpy(nbuf, name_ptr, name_len);
                    write_i32(target, @bitCast(nbuf));
                    write_i32(target + 4, @bitCast(name_len));
                    write_i32(target + 8, @bitCast(hash));
                    var k: u32 = HEADER_SIZE;
                    while (k < bucket_size) : (k += 4) {
                        write_i32(target + k, 0);
                    }
                    self.count += 1;
                    return target;
                }
                if (ep == TOMBSTONE) {
                    if (first_tombstone == 0) first_tombstone = base;
                } else {
                    // Live entry — check if it's our key (would be a
                    // logic error since callers should call ``find``
                    // first, but be defensive).
                    const el: u32 = @bitCast(read_i32(base + 4));
                    const eh: u32 = @bitCast(read_i32(base + 8));
                    if (eh == hash and el == name_len) {
                        // Empty-string key fast-path — see ``find``.
                        if (el == 0) return base;
                        const sp: [*]const u8 = @ptrFromInt(ep);
                        const np: [*]const u8 = @ptrFromInt(name_ptr);
                        var match = true;
                        for (0..el) |k| {
                            if (sp[k] != np[k]) {
                                match = false;
                                break;
                            }
                        }
                        if (match) return base;
                    }
                }
                idx = (idx + 1) & mask;
            }
            // Probe chain full of live + tombstones; reuse a tombstone
            // if one was seen.
            if (first_tombstone != 0) {
                const nbuf = alloc(name_len);
                // OOM guard mirrors the end-of-probe-chain path above.
                if (nbuf == 0 and name_len != 0) return null;
                memcpy(nbuf, name_ptr, name_len);
                write_i32(first_tombstone, @bitCast(nbuf));
                write_i32(first_tombstone + 4, @bitCast(name_len));
                write_i32(first_tombstone + 8, @bitCast(hash));
                var k: u32 = HEADER_SIZE;
                while (k < bucket_size) : (k += 4) {
                    write_i32(first_tombstone + k, 0);
                }
                self.count += 1;
                return first_tombstone;
            }
            return null;
        }

        /// 75% load-factor check.  Caller decides whether to grow.
        pub fn needs_grow(self: *const Self) bool {
            return self.count * 4 >= self.cap * 3;
        }

        /// Double capacity and rehash every entry.  Bucket addresses
        /// change — invalidate any external caches that reference
        /// bucket bases (e.g. LRU caches) BEFORE calling grow, then
        /// they'll be rebuilt lazily on the next lookup.
        pub fn grow(self: *Self) void {
            const old_buf = self.buf;
            const old_cap = self.cap;
            self.cap = old_cap * 2;
            self.buf = alloc(self.cap * bucket_size);
            self.count = 0;
            var i: u32 = 0;
            while (i < self.cap) : (i += 1) {
                write_i32(self.buf + i * bucket_size, 0);
            }
            i = 0;
            while (i < old_cap) : (i += 1) {
                const base = old_buf + i * bucket_size;
                const ep: u32 = @bitCast(read_i32(base));
                if (ep == 0 or ep == TOMBSTONE) continue;
                const eh: u32 = @bitCast(read_i32(base + 8));
                const mask = self.cap - 1;
                var idx = eh & mask;
                while (true) {
                    const nb = self.buf + idx * bucket_size;
                    if (@as(u32, @bitCast(read_i32(nb))) == 0) {
                        // Bulk-copy all bucket bytes (header + value)
                        // word-by-word.  ``bucket_size`` is comptime
                        // so this loop is fully unrolled in release.
                        // The ``name_ptr`` byte buffer is transferred
                        // verbatim — both old and new bucket share
                        // the same heap allocation, so we MUST NOT
                        // free it here (the new bucket still uses it).
                        var k: u32 = 0;
                        while (k < bucket_size) : (k += 4) {
                            write_i32(nb + k, read_i32(base + k));
                        }
                        self.count += 1;
                        break;
                    }
                    idx = (idx + 1) & mask;
                }
            }
            // Free the old bucket array backbone — the per-entry
            // name buffers were transferred to the new buffer above,
            // so only the table itself needs reclaiming.  Without
            // this every grow leaked ``old_cap * bucket_size`` bytes.
            obj.free_sized(old_buf, old_cap * bucket_size);
        }

        /// Iterate every populated bucket base in capacity order
        /// (insertion order is NOT preserved; addresses are stable
        /// only between grows).  Used by callers that need to walk
        /// every entry (e.g. namespace-fallback scan, ``info commands``).
        pub fn each(
            self: *const Self,
            ctx: anytype,
            comptime visit: fn (@TypeOf(ctx), u32) void,
        ) void {
            if (self.buf == 0) return;
            var i: u32 = 0;
            while (i < self.cap) : (i += 1) {
                const base = self.buf + i * bucket_size;
                const ep: u32 = @bitCast(read_i32(base));
                if (ep != 0 and ep != TOMBSTONE) visit(ctx, base);
            }
        }

        /// Mark the bucket at ``base`` as a tombstone.  ``base`` must
        /// be a return value from ``find`` (i.e. point at a live
        /// bucket header).  Decrements ``count`` so load-factor checks
        /// stay accurate; the underlying slot stays reserved against
        /// the probe chain until a re-insert reuses it.
        ///
        /// Lookups on the deleted key now miss; any external pointer
        /// into the bucket's value payload becomes invalid.
        pub fn delete_at(self: *Self, base: u32) void {
            const ep: u32 = @bitCast(read_i32(base));
            if (ep == 0 or ep == TOMBSTONE) return;
            // Free the name buffer that ``try_insert_header`` minted
            // — the tombstone marker overwrites the pointer, so the
            // slab would otherwise leak.  Capture the length before
            // we zero the field below.
            const el: u32 = @bitCast(read_i32(base + 4));
            write_i32(base, @bitCast(TOMBSTONE));
            // Optional: zero name_len + hash so a stale read sees
            // self-consistent zero rather than partial old data.
            write_i32(base + 4, 0);
            write_i32(base + 8, 0);
            if (self.count > 0) self.count -= 1;
            if (el > 0) obj.free_sized(ep, el);
        }
    };
}
