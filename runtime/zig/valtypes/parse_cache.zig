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
// * Storage is a ``hash_table.Table(16)`` keyed by the 8-byte
//   ``(body_ptr, body_len)`` tuple — the body's absolute address
//   in linear memory plus its length.  The tuple is staged into
//   a small module-level scratch buffer on every ``lookup`` /
//   ``insert`` and the hash-table primitive stores a fixed 8-byte
//   heap copy as the bucket key (independent of body size).
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
const free_sized = obj.free_sized;
const memcpy = obj.memcpy;
const read_i32 = obj.read_i32;
const write_i32 = obj.write_i32;

const ht = @import("hash_table.zig");
const parse = @import("../parse/tcl_parse.zig");

const BUCKET_SIZE: u32 = 16;
const INITIAL_CAP: u32 = 32;

/// Hard cap on live cache entries.  ``insert`` bulk-wipes the cache
/// when an attempted insert would push ``count`` past this value.
/// Not a true LRU: the underlying open-addressed table doesn't
/// support tombstones, so evicting one entry at a time would break
/// probe chains.  Bulk wipe keeps the implementation simple and
/// bounds memory against long-running embedders that build many
/// short-lived procs.  At ~1.6 KB per slab, 256 entries cap the
/// cache's own memory footprint at roughly 0.5 MB plus a small
/// bucket table.
///
/// For tcltest and similar bundles (≈50–120 registered procs) the
/// cap is never reached in practice.
const MAX_ENTRIES: u32 = 256;

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

// The key in each bucket is the 8-byte ``(body_ptr, body_len)``
// tuple — NOT the body bytes themselves.  Hashing the body bytes
// would force ``Table.insert_header`` to heap-copy the entire
// body (up to ``MAX_CACHED_BODY_LEN`` = 4 KB) as the bucket's
// name slot, duplicating memory that's already pinned by the
// ``Command`` struct that registered the proc.  Instead we stage
// the tuple into a small module-level scratch and pass its
// address + fixed length 8 to the hash-table primitive; the
// copy it stores is 8 bytes per entry regardless of body size.
const KEY_SIZE: u32 = 8;
var key_scratch: [KEY_SIZE]u8 align(4) = undefined;

fn stage_key(body_ptr: u32, body_len: u32) u32 {
    const addr: u32 = @intFromPtr(&key_scratch[0]);
    write_i32(addr, @bitCast(body_ptr));
    write_i32(addr + 4, @bitCast(body_len));
    return addr;
}

const OFF_SLAB_ADDR: u32 = ht.HEADER_SIZE;

const CacheTable = ht.Table(BUCKET_SIZE);
var cache: CacheTable = .{};

/// Find the cached slab address for a given body buffer, or 0 if
/// absent.  Caller treats the return value as a ``*Slab`` whose
/// header gives ``n_commands`` and ``total_tokens``.
///
/// Keyed on the 8-byte ``(body_ptr, body_len)`` tuple — distinct
/// body_ptr values (e.g. re-allocated TclObj string reprs for a
/// redefined proc body) always produce distinct entries even if
/// the content happens to match byte-for-byte.
pub fn lookup(body_ptr: u32, body_len: u32) u32 {
    if (cache.buf == 0) return 0;
    const key = stage_key(body_ptr, body_len);
    const hash = ht.fnv1a(key, KEY_SIZE);
    if (cache.find(key, KEY_SIZE, hash)) |bucket| {
        return @bitCast(read_i32(bucket + OFF_SLAB_ADDR));
    }
    return 0;
}

/// Compute the byte size of an in-cache slab from its header.  Used
/// by the invalidation paths to hand the slab back to ``free_sized``
/// with the same size class ``alloc_slab`` requested — without this,
/// every invalidated entry leaked one slab of up to ~1.6 KB
/// (issue #317).
fn slab_size(slab_addr: u32) u32 {
    const n_commands: u32 = @bitCast(read_i32(slab_addr + OFF_SLAB_N_COMMANDS));
    const total_tokens: u32 = @bitCast(read_i32(slab_addr + OFF_SLAB_TOTAL_TOKENS));
    return SLAB_HEADER_SIZE
        + n_commands * COMMAND_RECORD_SIZE
        + total_tokens * TOKEN_SIZE;
}

/// Free the slab pointed at by *bucket* and zero the slot.  No-op
/// when the bucket is empty / tombstoned or has no slab attached.
///
/// ``hash_table.Table.init`` / ``grow`` only zero each bucket's
/// ``name_ptr`` slot — the rest of the bucket payload (including
/// ``OFF_SLAB_ADDR``) carries whatever bytes ``alloc`` returned.
/// Reading the slab pointer from an empty bucket and handing it to
/// ``free_sized`` would be a wild free; gate on the occupied check
/// instead (Codex review on PR #322).
fn free_bucket_slab(bucket: u32) void {
    const np: u32 = @bitCast(read_i32(bucket));
    if (np == 0 or np == ht.TOMBSTONE) return;
    const slab: u32 = @bitCast(read_i32(bucket + OFF_SLAB_ADDR));
    if (slab == 0) return;
    write_i32(bucket + OFF_SLAB_ADDR, 0);
    free_sized(slab, slab_size(slab));
}

/// Insert a fresh ``(body_ptr, body_len) -> slab_addr`` entry.
/// Overwrites any existing entry for the same key (re-registering
/// the same body ptr with a re-parsed slab).  Caller must own
/// ``slab_addr`` (typically returned from
/// :func:`alloc_slab_for_body`).
pub fn insert(body_ptr: u32, body_len: u32, slab_addr: u32) void {
    cache.init(INITIAL_CAP);
    const key = stage_key(body_ptr, body_len);
    const hash = ht.fnv1a(key, KEY_SIZE);
    if (cache.find(key, KEY_SIZE, hash)) |bucket| {
        // Issue #317: free the old slab before overwriting; the
        // older code dropped the pointer without ``free_sized``,
        // leaking one slab on every re-parse for the same body.
        free_bucket_slab(bucket);
        write_i32(bucket + OFF_SLAB_ADDR, @bitCast(slab_addr));
        return;
    }
    // Bulk eviction on overflow — see MAX_ENTRIES docstring.
    if (cache.count >= MAX_ENTRIES) invalidate_all();
    if (cache.needs_grow()) cache.grow();
    const bucket = cache.insert_header(key, KEY_SIZE, hash);
    write_i32(bucket + OFF_SLAB_ADDR, @bitCast(slab_addr));
}

/// Invalidate any cache entry whose body_ptr equals ``buf_ptr``.
/// Called from :func:`tcl_obj.release_now` immediately before
/// ``free_sized`` releases a string buffer back to libc malloc —
/// otherwise a subsequent ``alloc`` could re-issue the slab to a
/// new TclObj whose ``(body_ptr, body_len)`` happens to match a
/// stale entry, returning tokens that point into freed memory.
///
/// Walk is O(cache.cap), but the cap is small (≤ MAX_ENTRIES *
/// load factor) and free is itself a relatively rare event.
/// Marks matched buckets as tombstones so probe chains for other
/// keys stay intact (see ``hash_table.TOMBSTONE``).
pub fn invalidate_for_buffer(buf_ptr: u32) void {
    if (cache.buf == 0) return;
    if (buf_ptr == 0 or buf_ptr == ht.TOMBSTONE) return;
    var i: u32 = 0;
    while (i < cache.cap) : (i += 1) {
        const bucket = cache.buf + i * BUCKET_SIZE;
        const np: u32 = @bitCast(read_i32(bucket));
        if (np == 0 or np == ht.TOMBSTONE) continue;
        const cached_body_ptr: u32 = @bitCast(read_i32(np));
        if (cached_body_ptr == buf_ptr) {
            // Issue #317: free the slab itself before tomb-stoning
            // the bucket; the older code zeroed the slab pointer
            // without handing the slab back to ``free_sized``, so
            // every unique proc body the cache had ever held leaked
            // one slab (~1.6 KB each) for the lifetime of the
            // process.
            free_bucket_slab(bucket);
            cache.delete_at(bucket);
        }
    }
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
    //
    // Issue #317: free each live slab before wiping the bucket.
    // The bulk-eviction path (called from :func:`insert` when the
    // cache hits ``MAX_ENTRIES``) and the proc-redefinition hook
    // both reach here; without ``free_sized`` every wipe leaked
    // up to ``MAX_ENTRIES * ~1.6 KB`` per call.
    var i: u32 = 0;
    while (i < cache.cap) : (i += 1) {
        const bucket = cache.buf + i * BUCKET_SIZE;
        free_bucket_slab(bucket);
        write_i32(bucket, 0);
        write_i32(bucket + 4, 0);
        write_i32(bucket + 8, 0);
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

/// Largest proc body we'll cache.  Bodies larger than this stay
/// on the cold parse-on-every-eval path — building a slab for
/// them would waste more bump-allocator memory than the saved
/// parse cost is worth.  A 4 KB cap covers every tcltest body
/// (largest is ~1.5 KB) with comfortable margin.
const MAX_CACHED_BODY_LEN: u32 = 4096;

/// Token buffer size for the pre-count pass.  Must exceed the
/// maximum tokens any single command can produce (2 * MAX_WORDS
/// on the interpreter side = 2 * 128 = 256).  We bump modestly
/// above that so a generous command still parses cleanly.
const SCAN_TOK_BUF: u32 = 384;

/// Parse ``body`` twice — first to count commands + tokens,
/// second to populate a cache slab — and insert the slab into
/// the cache keyed on ``(body_ptr, body_len)``.  Called from
/// ``proc_register`` / ``proc_register_compiled`` (P9.3) after a
/// successful registration, so subsequent ``eval_script`` calls
/// for the same body hit the warm path.
///
/// Bails silently on bodies larger than ``MAX_CACHED_BODY_LEN``,
/// on bodies that fail to parse, or on bodies whose total token
/// count exceeds the scratch buffer.  Either way the cold path
/// keeps working — the cache is best-effort.
pub fn build_for_body(body_ptr: u32, body_len: u32) void {
    if (body_len == 0 or body_len > MAX_CACHED_BODY_LEN) return;
    if (lookup(body_ptr, body_len) != 0) return; // already cached
    const src: [*]const u8 = @ptrFromInt(body_ptr);

    var tok_buf: [SCAN_TOK_BUF]parse.Token = undefined;

    // Pass 1: count commands + total tokens.
    var n_commands: u32 = 0;
    var total_tokens: u32 = 0;
    {
        var pos: u32 = 0;
        while (pos < body_len) {
            const cmd = parse.ParseCommand(src, pos, body_len, &tok_buf, tok_buf.len);
            pos = cmd.next;
            if (cmd.n_words == 0) continue;
            n_commands += 1;
            total_tokens += cmd.tokens_len;
        }
    }
    if (n_commands == 0) return;

    // Pass 2: allocate + populate.
    const slab = alloc_slab(n_commands, total_tokens);
    var rec_idx: u32 = 0;
    var tok_off: u32 = 0;
    {
        var pos: u32 = 0;
        while (pos < body_len) {
            const cmd = parse.ParseCommand(src, pos, body_len, &tok_buf, tok_buf.len);
            pos = cmd.next;
            if (cmd.n_words == 0) continue;
            const rec = command_record(slab, rec_idx);
            write_i32(rec + OFF_CR_TOKENS_OFFSET, @bitCast(tok_off));
            write_i32(rec + OFF_CR_TOKENS_LEN, @bitCast(cmd.tokens_len));
            write_i32(rec + OFF_CR_N_WORDS, @bitCast(cmd.n_words));
            write_i32(rec + OFF_CR_NEXT_POS, @bitCast(cmd.next));
            if (cmd.tokens_len > 0) {
                const tok_dst = token_at(slab, tok_off);
                memcpy(tok_dst, cmd.tokens_ptr, cmd.tokens_len * TOKEN_SIZE);
            }
            tok_off += cmd.tokens_len;
            rec_idx += 1;
        }
    }
    insert(body_ptr, body_len, slab);
}
