// Parse cache for interpreted proc bodies (Phase 9).
//
// ``eval_script`` re-parses every proc body on every call — for
// tcltest-style bundles that evaluate the same body dozens to
// thousands of times, the ``ParseCommand`` work dominates.  This
// module caches the parse result so subsequent evals can replay a
// pre-built command list instead of re-tokenising.
//
// Shape:
//
// * Storage is a ``hash_table.Table(16)`` keyed by ``(body_ptr,
//   body_len)`` — the body's absolute address in linear memory.
//   Bodies are TclObj string reprs pinned by the ``Command``
//   struct that registered the proc, so the address stays valid
//   for as long as the cache entry is live.
// * Each bucket value points at a heap-allocated "parsed slab"
//   whose layout is:
//
//       [ 0.. 3] n_commands    : u32
//       [ 4.. 7] total_tokens  : u32  (count of ``parse.Token`` entries)
//       [ 8..  ] CommandRecord × n_commands   (16 bytes each)
//       [   ...] parse.Token   × total_tokens (16 bytes each)
//
//   CommandRecord = ``(tokens_offset, tokens_len, n_words,
//   next_pos)`` — offsets into the token array and the source
//   body respectively.
//
// * Invalidation: wholesale reset on ``proc_register`` (belt-and-
//   suspenders — a proc redefinition leaves the old body
//   unreachable but a stray cache entry keyed on its address is
//   merely dead weight).  P9.2 wires the ``eval_script`` reader
//   path; P9.3 plumbs the invalidation hook.

const obj = @import("tcl_obj.zig");
const alloc = obj.alloc;
const memcpy = obj.memcpy;
const read_i32 = obj.read_i32;
const write_i32 = obj.write_i32;

const ht = @import("hash_table.zig");
const parse = @import("tcl_parse.zig");

const BUCKET_SIZE: u32 = 16;
const INITIAL_CAP: u32 = 32;

// Offsets of the four u32s that make up a ``CommandRecord``.
pub const OFF_CR_TOKENS_OFFSET: u32 = 0;
pub const OFF_CR_TOKENS_LEN: u32 = 4;
pub const OFF_CR_N_WORDS: u32 = 8;
pub const OFF_CR_NEXT_POS: u32 = 12;
pub const COMMAND_RECORD_SIZE: u32 = 16;

// Token slot size (must match ``parse.Token``'s layout).  The
// Zig runtime already assumes this elsewhere; this constant is
// here so the replay path can reconstruct token pointers by
// offset arithmetic without importing ``@sizeOf(parse.Token)``
// into every call site.
pub const TOKEN_SIZE: u32 = @sizeOf(parse.Token);

// Slab header offsets.
pub const OFF_SLAB_N_COMMANDS: u32 = 0;
pub const OFF_SLAB_TOTAL_TOKENS: u32 = 4;
pub const SLAB_HEADER_SIZE: u32 = 8;

// The key in each bucket is just ``(body_ptr, body_len)`` — the
// hash table's header already stores those as the bucket header's
// ``name_ptr`` / ``name_len``.  The payload (bucket value at
// offset ``HEADER_SIZE``) is the slab address.
const OFF_SLAB_ADDR: u32 = ht.HEADER_SIZE;

const CacheTable = ht.Table(BUCKET_SIZE);
var cache: CacheTable = .{};

/// Find the cached slab address for a given body buffer, or 0 if
/// absent.  Caller treats the return value as a ``*Slab`` whose
/// header gives ``n_commands`` and ``total_tokens``.
pub fn lookup(body_ptr: u32, body_len: u32) u32 {
    if (cache.buf == 0) return 0;
    // Quirk: hash_table.Table.find uses the name bytes as its key.
    // We want to key on ``(body_ptr, body_len)``, where body_ptr
    // is itself the absolute address of the key bytes — so we can
    // hand the table ``body_ptr`` as the key pointer and the
    // hash computed over those same bytes.  The bytes ARE the key.
    const hash = ht.fnv1a(body_ptr, body_len);
    if (cache.find(body_ptr, body_len, hash)) |bucket| {
        return @bitCast(read_i32(bucket + OFF_SLAB_ADDR));
    }
    return 0;
}

/// Insert a fresh ``(body_ptr, body_len) -> slab_addr`` entry.
/// Overwrites any existing entry for the same key (re-registering
/// the same body ptr with a re-parsed slab).  Caller must own
/// ``slab_addr`` (typically returned from
/// :func:`alloc_slab_for_body`).
pub fn insert(body_ptr: u32, body_len: u32, slab_addr: u32) void {
    cache.init(INITIAL_CAP);
    const hash = ht.fnv1a(body_ptr, body_len);
    if (cache.find(body_ptr, body_len, hash)) |bucket| {
        write_i32(bucket + OFF_SLAB_ADDR, @bitCast(slab_addr));
        return;
    }
    if (cache.needs_grow()) cache.grow();
    const bucket = cache.insert_header(body_ptr, body_len, hash);
    write_i32(bucket + OFF_SLAB_ADDR, @bitCast(slab_addr));
}

/// Wipe every cached entry.  Called from
/// :func:`tcl_procs.proc_register` (via P9.3) because a
/// redefinition may make old entries semantically stale for
/// callers that dispatch the new version with the same body
/// bytes (unusual but possible for identical-source reinserts).
pub fn invalidate_all() void {
    if (cache.buf == 0) return;
    // Simplest valid wipe: rewrite each bucket's ``name_ptr`` to
    // 0.  The ``count`` field stays as-is — the stored counter
    // isn't consulted by ``find`` (which stops at empty slots)
    // or ``needs_grow`` (which compares against cap).  Clearing
    // count lets future inserts start fresh.
    var i: u32 = 0;
    while (i < cache.cap) : (i += 1) {
        const bucket = cache.buf + i * BUCKET_SIZE;
        write_i32(bucket, 0);
        write_i32(bucket + 4, 0);
        write_i32(bucket + 8, 0);
        write_i32(bucket + OFF_SLAB_ADDR, 0);
    }
    cache.count = 0;
}

/// Allocate a slab sized for ``n_commands`` records plus
/// ``total_tokens`` tokens, with the header prewritten.  Returns
/// the slab address.  Caller fills in each ``CommandRecord`` and
/// the packed ``parse.Token`` array.
pub fn alloc_slab(n_commands: u32, total_tokens: u32) u32 {
    const size =
        SLAB_HEADER_SIZE
        + n_commands * COMMAND_RECORD_SIZE
        + total_tokens * TOKEN_SIZE;
    const addr = alloc(size);
    write_i32(addr + OFF_SLAB_N_COMMANDS, @bitCast(n_commands));
    write_i32(addr + OFF_SLAB_TOTAL_TOKENS, @bitCast(total_tokens));
    return addr;
}

/// Return the address of the i-th ``CommandRecord`` in a slab.
pub fn command_record(slab_addr: u32, i: u32) u32 {
    return slab_addr + SLAB_HEADER_SIZE + i * COMMAND_RECORD_SIZE;
}

/// Return the address of the first byte of the token area inside
/// a slab (following the records array).
pub fn token_area_start(slab_addr: u32) u32 {
    const n_commands: u32 = @bitCast(read_i32(slab_addr + OFF_SLAB_N_COMMANDS));
    return slab_addr + SLAB_HEADER_SIZE + n_commands * COMMAND_RECORD_SIZE;
}

/// Return the absolute address of the k-th token slot within a
/// slab's token area.
pub fn token_at(slab_addr: u32, token_index: u32) u32 {
    return token_area_start(slab_addr) + token_index * TOKEN_SIZE;
}

pub fn slab_n_commands(slab_addr: u32) u32 {
    return @bitCast(read_i32(slab_addr + OFF_SLAB_N_COMMANDS));
}
