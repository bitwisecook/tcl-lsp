// TclObj value type, memory management, and string helpers for WASM.
//
// Memory layout of a TclObj (32 bytes; ``OBJ_SIZE``):
//   offset 0:  refcount  (i32)
//   offset 4:  type_tag  (i32)  0=string, 1=int, 2=list, 3=dict,
//                                4=float, 5=inline-string
//   offset 8:  int_cache (i64)  cached integer / float bits, OR
//                                ≤ 8-byte inline string payload
//                                when type_tag == TYPE_INLINE_STRING
//   offset 16: str_ptr   (i32)  pointer to UTF-8 string-rep buffer
//                                (or to the inline payload at +8 for
//                                TYPE_INLINE_STRING)
//   offset 20: str_len   (i32)  byte length of the string rep
//   offset 24: str_cap   (i32)  size-class-rounded capacity of the
//                                buffer pointed to by ``str_ptr``;
//                                non-zero ⇒ obj owns the buffer and
//                                ``release_now`` will ``free_sized``
//                                it.  Zero ⇒ borrowed (interned
//                                literal in the data segment, or the
//                                inline payload at +8).
//   offset 28: dict_ext  (i32)  pointer to a dict hash side-cache
//                                (see ``tcl_dict.zig``); zero on
//                                every other obj type
//
// Layering: character classification and byte-span comparison are in
// ``tcl_chars.zig``; list-quoting / list-parsing / backslash decoding
// live in their own modules.  This file owns the TclObj memory model
// and integer / boolean scalar parsers.  ``pub const`` re-exports of
// ``is_space`` / ``is_scan_space`` / ``str_cmp`` are kept as
// compatibility shims so existing ``obj.is_space`` callers work —
// new code should import ``tcl_chars.zig`` directly.

const chars = @import("tcl_chars.zig");
const bignum = @import("tcl_bignum.zig");
const std = @import("std");
const build_options = @import("build_options");

// Type tags
pub const TYPE_STRING: i32 = 0;
pub const TYPE_INT: i32 = 1;
pub const TYPE_LIST: i32 = 2;
pub const TYPE_DICT: i32 = 3;
// Sentinel written into ``OBJ_TYPE_TAG`` of every freed slab before
// it joins a per-class free-list.  ``tcl_obj_release`` checks for it
// before reading ``OBJ_REFCOUNT`` (which now overlays the freelist
// next-link, not a real refcount), so a stale handle that points at
// a slab on the freelist becomes a survivable no-op rather than the
// "decrement the next-link" pattern that corrupted the chain and
// surfaced as an out-of-bounds memory fault inside ``obj_alloc``.
// ``obj_alloc`` overwrites the tag with ``TYPE_STRING`` when it
// reissues the slab, so live objects never collide with the marker.
pub const TYPE_FREED_POISON: i32 = -559038737; // 0xDEADBEEF
// TYPE_FLOAT: f64 bits stored in the int_cache field via @bitCast.
// The str_ptr/str_len fields hold a cached string representation
// once obj_ensure_string has been called.
pub const TYPE_FLOAT: i32 = 4;
// TYPE_INLINE_STRING: short strings (≤ MAX_INLINE_STR bytes) stored
// directly in the obj header at OBJ_INT_CACHE.  No external buffer
// is allocated, so :func:`obj_new_string_copy` for a short payload
// avoids the second ``alloc`` call.  ``obj_str_ptr`` returns a
// pointer INTO the obj header for these — readers must not hold
// the pointer past the obj's lifetime.  ``obj_type`` reports
// ``TYPE_STRING`` for callers that don't distinguish, so the
// inline encoding stays transparent to the rest of the system.
pub const TYPE_INLINE_STRING: i32 = 5;
pub const MAX_INLINE_STR: u32 = 8;
// TYPE_BIGNUM: arbitrary-precision integer (Stage 2).  The
// OBJ_DICT_EXT slot (offset 28) holds a ``*bignum.BigInt`` —
// a heap-allocated ``std.math.big.int.Managed`` whose limb
// buffer is owned through ``std.heap.c_allocator`` (wasi-libc
// malloc), keeping the bignum heap consistent with the rest of
// the runtime.  Stage 1 used a 16-byte off-heap i128 payload;
// Stage 2 lifted the i128 cap so values like
// ``665802003400000000000000`` (upstream ``expr-old-36.11``,
// 80 bits) and ``0xff..ff`` (38 hex digits, 152 bits, upstream
// ``expr-old-36.16``) round-trip exactly.
//
// The OBJ_DICT_EXT slot is shared with the dict hash side-
// cache; a single TclObj is one type at a time, so the slot's
// owning subsystem is determined by ``OBJ_TYPE_TAG``.
// ``release_now`` checks the tag and routes the cleanup either
// to ``bignum.destroy`` (TYPE_BIGNUM) or
// ``tcl_dict.dict_destroy_ext`` (TYPE_DICT).
//
// String-rep caching reuses the same ``OBJ_STR_PTR`` /
// ``OBJ_STR_LEN`` / ``OBJ_STR_CAP`` machinery as TYPE_INT and
// TYPE_FLOAT, so a value rendered once stays cached for the
// rest of the obj's lifetime.
pub const TYPE_BIGNUM: i32 = 6;

// TclObj field offsets
pub const OBJ_REFCOUNT: u32 = 0;
pub const OBJ_TYPE_TAG: u32 = 4;
pub const OBJ_INT_CACHE: u32 = 8;
pub const OBJ_STR_PTR: u32 = 16;
pub const OBJ_STR_LEN: u32 = 20;
// OBJ_STR_CAP: capacity of the buffer pointed to by OBJ_STR_PTR.
// Set to 0 when the buffer is not owned by this TclObj (e.g. points
// into a wasm data segment for an interned literal, or into a shared
// constant pool).  Set to >0 when this TclObj allocated its own
// buffer via ``alloc(cap)`` — in that case ``tcl_cmd_append`` and
// the recycler may free the buffer via ``free_sized(ptr, cap)``.
//
// The capacity field enables amortised O(1) ``append`` by letting
// the runtime grow the buffer geometrically (doubling) instead of
// reallocating on every append.
pub const OBJ_STR_CAP: u32 = 24;
// OBJ_DICT_EXT: pointer to a hash side-cache for dict objects (see
// :file:`tcl_dict.zig`).  Zero on every other obj type.  Co-located
// with the existing 32-byte header — bytes 28..31 were padding
// before the dict cache was introduced.
pub const OBJ_DICT_EXT: u32 = 28;
pub const OBJ_SIZE: u32 = 32;

// S6.4 — Tagged-immediate small integers.
//
// Bit 0 of an i32 handle is the tag bit.  When set, the handle is
// NOT a pointer to a TclObj header; it encodes a signed integer
// shifted left by 1 with the tag in bit 0:
//
//   handle = (value << 1) | 1
//   value  = handle >> 1   (arithmetic shift — sign-preserving)
//
// Range: ``[0, 2^30 - 1]`` = ``[0, 1_073_741_823]``.  Negative
// values fall back to allocation.
//
// Why the range is positive-only (PR #237 review): the frame
// layer's local-variable bucket value field shares the same i32
// space, and uses negative sentinels to mark variable aliases:
//
//   * ``ALIAS_GLOBAL`` = ``-1``  (bucket value -1 means "alias to
//     a same-named global").
//   * ``ALIAS_EXT``    = ``v <= -2`` (bucket value is the negated
//     descriptor heap address).
//
// A tagged immediate of ``-1`` encodes as ``(0xFFFFFFFE | 1) =
// 0xFFFFFFFF = -1`` — the same bit pattern as ``ALIAS_GLOBAL``.
// Any negative value's encoding has bit 31 set, so it can also
// collide with a descriptor-pointer sentinel.  Restricting the
// tagged-immediate range to **non-negative** values makes the
// encoding bit-31 = 0, which can't ever equal a negative
// alias sentinel.  The cost is that small *negative* integers
// (``-1``, ``-2``, …) take the heap-allocated path, but loop
// counters, list indices, and the vast majority of literal
// integers are non-negative anyway.
//
// Why bit 0 is safe as a tag:
//
// 1. The allocator rounds every request up to a multiple of 8
//    (``alloc`` does ``(size + 7) & ~7`` and the heap starts on a
//    page boundary), so every valid TclObj pointer has bits 0..2
//    = 0.  A handle with bit 0 set cannot collide with a valid
//    object address.
// 2. ``handle == 0`` (null) is preserved: bit 0 is 0, so it is
//    treated as a pointer (and null-checked separately by every
//    accessor).
//
// Tagged immediates are immortal — they have no refcount and no
// header, so :func:`tcl_obj_retain` / :func:`tcl_obj_release` are
// no-ops on them.  Readers (:func:`obj_get_int`, :func:`obj_type`,
// :func:`obj_ensure_string`) detect the tag bit via
// :func:`is_immediate` and short-circuit before dereferencing.

pub const IMMEDIATE_TAG_BIT: u32 = 0x1;
pub const IMMEDIATE_MIN: i64 = 0;
pub const IMMEDIATE_MAX: i64 = (1 << 30) - 1;

/// True iff *handle* is a tagged immediate (bit 0 set).
pub fn is_immediate(handle: i32) bool {
    return (@as(u32, @bitCast(handle)) & IMMEDIATE_TAG_BIT) != 0;
}

/// Return the boxed signed integer represented by a tagged immediate
/// handle.  Caller must already have verified ``is_immediate(handle)``.
pub fn immediate_unbox(handle: i32) i64 {
    return @as(i64, handle >> 1);
}

/// Encode *value* as a tagged-immediate handle, or return ``null``
/// when it doesn't fit the 31-bit signed range.
pub fn immediate_box(value: i64) ?i32 {
    if (value < IMMEDIATE_MIN or value > IMMEDIATE_MAX) return null;
    const v32: i32 = @intCast(value);
    const u: u32 = @bitCast(v32);
    return @bitCast((u << 1) | IMMEDIATE_TAG_BIT);
}

// WASM linear-memory allocator.
//
// Layout:
//   - bump pointer ``heap_ptr`` starts at the first 64 KB page boundary
//     above the data segment (initialised to 65536 by convention)
//   - per-size-class free-lists for recycled slabs (sub-plan 1.2 will
//     extend the single OBJ_SIZE list to a vector of size classes)
//   - calls ``@wasmMemoryGrow`` when ``heap_ptr + size`` would cross
//     the current memory size, instead of unconditionally bumping past
//     the limit (the previous behaviour caused ``out of bounds memory
//     access`` traps once allocations exceeded the initial linear-memory
//     reservation).
//
// One WASM memory page is 64 KB (PAGE_SIZE).  ``@wasmMemorySize(0)``
// returns the current page count; ``@wasmMemoryGrow(0, n)`` requests
// ``n`` more pages and returns the previous page count, or -1 on
// failure.
//
// A configurable cap (``MAX_HEAP_PAGES``) bounds total memory; reaching
// it raises a Tcl-friendly "out of memory" via the runtime's catch
// machinery rather than letting a raw wasm trap surface.

pub const PAGE_SIZE: u32 = 65536;
pub const MAX_HEAP_PAGES: u32 = 4096; // 256 MB ceiling; configurable via build flag in future

// Allocator coherence (Phase 1.x — replaces the previous heap_ptr
// bump allocator):
//
// We share wasm linear memory with both the runtime's own static
// data segment AND the regex engine's wasi-libc malloc heap.  The
// previous bump allocator started at a fixed offset (16-80 MB,
// chosen to clear data segments) and grew upward via
// ``@wasmMemoryGrow``.  Problem: wasi-libc's dlmalloc *also*
// grows upward in the same memory, with no separation, so on
// heavy regex workloads (regexp.test compiles many regex_t,
// each malloc'ing a guts struct + sub-allocations) the two heaps
// eventually overlap and corrupt each other's free-list metadata.
// Symptom: ``out of bounds memory access at 0x676e6964`` (text
// bytes "ding" interpreted as a free-list pointer) deep inside
// ``c.malloc.malloc``.
//
// Fix: route every ``alloc()`` through wasi-libc ``malloc`` so
// the two consumers share one coherent heap.  ``free_sized`` /
// ``free_obj`` route through ``free``.  This trades the bump
// allocator's "no per-call cost" for libc's malloc — overhead is
// a few hundred ns per call but tcltest workloads spend most of
// their time elsewhere (parser, dispatch) so the wall-time hit
// is negligible.  In return we get correctness on every
// regex-heavy workload and lose a class of latent corruption
// bugs.
//
// S6.1: re-enabled the size-class free-lists for the four most-
// common size classes (32 / 48 / 64 / 96 — TclObj headers + small
// strings).  ``alloc()`` pops a slab if available before falling
// through to libc malloc; ``free_obj`` / ``free_sized`` push.
// Each list is bounded at MAX_FREELIST entries so a regex stress
// test cannot pin too much memory in the recycler.
//
// The four classes cover the bulk of allocs in tcltest workloads;
// classes above 96 still go straight to libc.

extern fn malloc(size: usize) ?*anyopaque;
extern fn free(ptr: ?*anyopaque) void;
extern fn calloc(nmemb: usize, size: usize) ?*anyopaque;

var oom_flag: u32 = 0;

// Per-size-class free-list head pointers.  When non-zero, the
// slab at the head's address contains its successor's address at
// offset 0 (we re-purpose the first word of a free slab as the
// next-link).  Only the first four classes are recycled today.
const SIZE_CLASSES = [_]u32{ 32, 48, 64, 96, 128, 192, 256, 384, 512, 1024, 2048 };
const FREELIST_REUSE_CLASSES: u32 = 4; // first 4 classes recycle
const MAX_FREELIST: u32 = 256;
var free_lists = [_]u32{0} ** SIZE_CLASSES.len;
var free_list_counts = [_]u32{0} ** SIZE_CLASSES.len;

fn class_head_for(aligned: u32) ?*u32 {
    for (SIZE_CLASSES, 0..) |sz, i| {
        if (sz == aligned) return &free_lists[i];
    }
    return null;
}

pub fn round_up_to_class(aligned: u32) u32 {
    for (SIZE_CLASSES) |sz| {
        if (sz >= aligned) return sz;
    }
    return aligned; // larger than the largest class — return as-is
}

pub export fn alloc(size: u32) callconv(.c) u32 {
    const requested = (size + 7) & ~@as(u32, 7);
    const aligned = if (requested <= 2048) round_up_to_class(requested) else requested;
    // S6.1: recycle from the per-class free-list if it has a slab
    // available.  Only the first four classes participate so
    // larger allocations still go straight to libc.
    for (SIZE_CLASSES[0..FREELIST_REUSE_CLASSES], 0..) |cls, i| {
        if (cls == aligned and free_lists[i] != 0) {
            const head = free_lists[i];
            // Read the next-link (overlays the slab's first word).
            const next: u32 = @bitCast(read_i32(head));
            free_lists[i] = next;
            free_list_counts[i] -= 1;
            return head;
        }
    }
    const p = malloc(@intCast(aligned)) orelse {
        oom_flag = 1;
        return 0;
    };
    return @intCast(@intFromPtr(p));
}

/// Push a slab onto the free-list for its size class if there's
/// room and the size matches one of the recycled classes.  Returns
/// True if the slab was kept; False if the caller should hand it
/// to libc free.
fn try_recycle(addr: u32, size: u32) bool {
    const aligned = if (size <= 2048) round_up_to_class((size + 7) & ~@as(u32, 7)) else size;
    for (SIZE_CLASSES[0..FREELIST_REUSE_CLASSES], 0..) |cls, i| {
        if (cls == aligned and free_list_counts[i] < MAX_FREELIST) {
            // Push: store current head as next-link in the slab,
            // then set head to this slab.
            write_i32(addr, @bitCast(free_lists[i]));
            free_lists[i] = addr;
            free_list_counts[i] += 1;
            return true;
        }
    }
    return false;
}

pub export fn tcl_test_heap_ptr() i32 {
    // No longer meaningful under the libc-routed allocator; kept
    // as an export for backwards compat with tests that probed
    // the old bump-pointer state.  Returns 0.
    return 0;
}

/// True if any ``alloc`` call observed an out-of-memory condition since
/// the last reset.  Reactor entry-points should clear it via
/// ``tcl_oom_clear`` at the start of each invocation; the runtime checks
/// it at strategic points (loop bodies, before host bridges) and
/// converts to a Tcl error.
pub export fn tcl_oom_get() i32 {
    return @bitCast(oom_flag);
}

pub export fn tcl_oom_clear() void {
    oom_flag = 0;
}

fn free_obj(addr: u32) void {
    if (addr == 0) return;
    // Poison the type tag so a stale ``tcl_obj_release`` on the
    // freed slab can detect the use-after-free and skip the
    // decrement-and-write that would otherwise corrupt the freelist
    // next-link.  ``obj_alloc`` writes ``TYPE_STRING`` over the
    // poison when the slab is reissued, so the marker only lives
    // for the slab's freelist sojourn.
    write_i32(addr + OBJ_TYPE_TAG, TYPE_FREED_POISON);
    // S6.1: TclObj headers are size class 32 (OBJ_SIZE = 32) so
    // try the recycler first.
    if (try_recycle(addr, OBJ_SIZE)) return;
    free(@ptrFromInt(addr));
}

/// Public free-by-size: routes through wasi-libc ``free``.  The
/// ``size`` argument was historically ignored under the libc-only
/// path; S6.1 uses it to decide whether the slab is small enough
/// to recycle into the per-class free-list before falling through
/// to libc.  Callers that pass ``size = 0`` or with addr == 0 are
/// no-ops.
pub fn free_sized(addr: u32, size: u32) void {
    if (addr == 0) return;
    if (size > 0 and try_recycle(addr, size)) return;
    free(@ptrFromInt(addr));
}

pub fn read_i32(addr: u32) i32 {
    const ptr: [*]const u8 = @ptrFromInt(addr);
    const bytes = [4]u8{ ptr[0], ptr[1], ptr[2], ptr[3] };
    return @bitCast(bytes);
}

pub fn write_i32(addr: u32, val: i32) void {
    const ptr: [*]u8 = @ptrFromInt(addr);
    const bytes: [4]u8 = @bitCast(val);
    ptr[0] = bytes[0];
    ptr[1] = bytes[1];
    ptr[2] = bytes[2];
    ptr[3] = bytes[3];
}

pub fn read_i64(addr: u32) i64 {
    const ptr: [*]const u8 = @ptrFromInt(addr);
    const bytes = [8]u8{
        ptr[0], ptr[1], ptr[2], ptr[3],
        ptr[4], ptr[5], ptr[6], ptr[7],
    };
    return @bitCast(bytes);
}

pub fn write_i64(addr: u32, val: i64) void {
    const ptr: [*]u8 = @ptrFromInt(addr);
    const bytes: [8]u8 = @bitCast(val);
    inline for (0..8) |i| {
        ptr[i] = bytes[i];
    }
}

// -- Leak-check counters (S0.2) ------------------------------------
//
// Bumped only when ``-Dleak-check=true`` is set on the zig build
// command line.  Production builds compile out the bookkeeping
// entirely (the ``if (build_options.leak_check)`` is folded by the
// optimiser to a constant ``false``).
//
// The harness reads ``tcl_test_alloc_count`` after a workload to
// assert zero residual.  ``tcl_test_double_free_count`` catches the
// case where ``tcl_obj_release`` is called on an obj whose refcount
// is already 0 (already on the deferred-free queue) — that is the
// over-release pattern S2's failed attempt hit.

var g_alloc_count: i32 = 0;
var g_double_free_count: i32 = 0;

pub export fn tcl_test_alloc_count() i32 {
    return g_alloc_count;
}

pub export fn tcl_test_double_free_count() i32 {
    return g_double_free_count;
}

pub export fn tcl_test_reset_counters() void {
    g_alloc_count = 0;
    g_double_free_count = 0;
}

/// Run at the end of a test workload.  Drains the deferred-free
/// queue first (so any objs queued but not yet freed do not show up
/// as residual), then returns the residual alloc count.  The
/// harness compares this to zero and reports the delta.
pub export fn tcl_test_finalize() i32 {
    tcl_obj_drain_pending();
    return g_alloc_count;
}

// Allocate a new TclObj with refcount 1
pub fn obj_alloc() u32 {
    const ptr = alloc(OBJ_SIZE);
    // Out-of-memory at the slab layer surfaces as a 0 return.  The
    // header writes below would panic on the null cast in debug
    // builds; bail early so the caller's null-check (every
    // ``obj_new_*`` site below) can convert the OOM into a
    // catchable Tcl error rather than a hard trap.
    if (ptr == 0) return 0;
    if (build_options.leak_check) g_alloc_count += 1;
    write_i32(ptr + OBJ_REFCOUNT, 1);
    write_i32(ptr + OBJ_TYPE_TAG, TYPE_STRING);
    write_i64(ptr + OBJ_INT_CACHE, 0);
    write_i32(ptr + OBJ_STR_PTR, 0);
    write_i32(ptr + OBJ_STR_LEN, 0);
    // Default cap = 0 means "we don't own the buffer".  Buffer-
    // owning paths (obj_new_string_copy, the in-place append grower)
    // overwrite this with the actual allocation size.
    write_i32(ptr + OBJ_STR_CAP, 0);
    write_i32(ptr + OBJ_DICT_EXT, 0);
    return ptr;
}

pub export fn obj_new_int(value: i64) i32 {
    // S6.4 — short-circuit small ints to a tagged immediate.
    // No allocation, no refcount, no header.  Most loop counters
    // and small literals (0, 1, -1, …) take this path.
    if (immediate_box(value)) |tagged| return tagged;
    const ptr = obj_alloc();
    if (ptr == 0) return 0; // OOM — alloc() set oom_flag.
    write_i32(ptr + OBJ_TYPE_TAG, TYPE_INT);
    write_i64(ptr + OBJ_INT_CACHE, value);
    // ``@bitCast`` (not ``@intCast``) — TclObj handles are opaque
    // bit patterns; allocator addresses above 2 GiB are valid in
    // wasm32 (4 GiB linear-memory range) and must round-trip
    // through the i32 handle slot without a debug-build overflow
    // panic.  Matches the reader sites which use ``@bitCast`` for
    // ``i32 handle → u32 addr``.  See issue #303.
    return @bitCast(ptr);
}

pub export fn obj_new_string(data_ptr: i32, length: i32) i32 {
    const ptr = obj_alloc();
    if (ptr == 0) return 0; // OOM — alloc() set oom_flag.
    write_i32(ptr + OBJ_TYPE_TAG, TYPE_STRING);
    write_i32(ptr + OBJ_STR_PTR, data_ptr);
    write_i32(ptr + OBJ_STR_LEN, length);
    return @bitCast(ptr);
}

pub export fn obj_new_float(value: f64) i32 {
    const ptr = obj_alloc();
    if (ptr == 0) return 0; // OOM — alloc() set oom_flag.
    write_i32(ptr + OBJ_TYPE_TAG, TYPE_FLOAT);
    write_i64(ptr + OBJ_INT_CACHE, @bitCast(value));
    return @bitCast(ptr);
}

/// Allocate a new TYPE_BIGNUM TclObj holding *value*.  Values
/// that fit in i64 collapse to TYPE_INT (via :func:`obj_new_int`)
/// so callers don't accidentally pay the bignum allocation cost
/// for an already-small result — overflow-detection in the
/// arithmetic helpers is the right place to choose between the
/// two representations.
///
/// Stage 2 storage: an off-heap ``bignum.BigInt`` (a Zig
/// ``std.math.big.int.Managed``).  The pointer is written into
/// ``OBJ_DICT_EXT``; ``release_now`` detects TYPE_BIGNUM and
/// routes cleanup through ``bignum.destroy``.  String-rep caching
/// reuses ``OBJ_STR_PTR`` / ``OBJ_STR_LEN`` / ``OBJ_STR_CAP``
/// like other scalar types.
pub fn obj_new_bignum(value: i128) i32 {
    // Promote-down to TYPE_INT when the value is i64-representable.
    if (value >= std.math.minInt(i64) and value <= std.math.maxInt(i64)) {
        return obj_new_int(@intCast(value));
    }
    const m = bignum.alloc_from_int(value) orelse return 0;
    return obj_new_bignum_take(m);
}

/// Take ownership of an already-allocated ``*bignum.BigInt`` and
/// wrap it in a TclObj.  Auto-collapses to TYPE_INT (and frees the
/// BigInt) when the value fits in i64 — saves a heap allocation for
/// every "result happens to fit i64" arithmetic case.
///
/// Caller transfers ownership: on success, the obj owns *m* and
/// will free it on release.  On obj-allocation failure, *m* is
/// destroyed before returning 0 so the caller can't double-free.
pub fn obj_new_bignum_take(m: *bignum.BigInt) i32 {
    if (bignum.fits_i64(m)) {
        const v = bignum.to_i64(m);
        bignum.destroy(m);
        return obj_new_int(v);
    }
    const ptr = obj_alloc();
    if (ptr == 0) {
        bignum.destroy(m);
        return 0;
    }
    write_i32(ptr + OBJ_TYPE_TAG, TYPE_BIGNUM);
    write_i32(ptr + OBJ_DICT_EXT, @bitCast(@as(u32, @intCast(@intFromPtr(m)))));
    return @bitCast(ptr);
}

/// Read the underlying ``*bignum.BigInt`` from a TYPE_BIGNUM obj,
/// or ``null`` for any other tag.  The returned pointer is borrowed
/// — callers must not destroy it (the obj retains ownership).
pub fn obj_get_bignum_managed(obj: i32) ?*bignum.BigInt {
    if (obj == 0) return null;
    if (is_immediate(obj)) return null;
    const addr: u32 = @bitCast(obj);
    const tag = read_i32(addr + OBJ_TYPE_TAG);
    if (tag != TYPE_BIGNUM) return null;
    const ptr_addr: u32 = @bitCast(read_i32(addr + OBJ_DICT_EXT));
    if (ptr_addr == 0) return null;
    return @ptrFromInt(ptr_addr);
}

/// Read an i128 view of any int-shaped TclObj.  Truncates to the
/// low 128 bits when the value exceeds i128 — callers needing the
/// full magnitude must go through :func:`obj_get_bignum_managed`
/// (when ``obj_type(obj) == TYPE_BIGNUM``).
pub fn obj_get_bignum(obj: i32) i128 {
    if (obj == 0) return 0;
    if (is_immediate(obj)) return @as(i128, immediate_unbox(obj));
    const addr: u32 = @bitCast(obj);
    const tag = read_i32(addr + OBJ_TYPE_TAG);
    if (tag == TYPE_BIGNUM) {
        const ptr_addr: u32 = @bitCast(read_i32(addr + OBJ_DICT_EXT));
        if (ptr_addr == 0) return 0;
        const m: *bignum.BigInt = @ptrFromInt(ptr_addr);
        return bignum.to_i128(m);
    }
    if (tag == TYPE_INT) return @as(i128, read_i64(addr + OBJ_INT_CACHE));
    if (tag == TYPE_FLOAT) {
        const fval: f64 = @bitCast(read_i64(addr + OBJ_INT_CACHE));
        return float_to_i128_clamped(fval);
    }
    if (tag == TYPE_STRING or tag == TYPE_INLINE_STRING) {
        const sptr: u32 = @bitCast(read_i32(addr + OBJ_STR_PTR));
        const slen: u32 = @bitCast(read_i32(addr + OBJ_STR_LEN));
        if (bignum.parse_i128(sptr, slen)) |val| return val;
        if (try_parse_int(sptr, slen)) |val| return @as(i128, val);
        if (try_parse_float(sptr, slen)) |fval| return float_to_i128_clamped(fval);
    }
    return 0;
}

/// Convert *fval* to ``i128``, clamping to the i128 range and
/// returning 0 for NaN.  Plain ``@intFromFloat`` panics in debug
/// builds when *fval* is outside the destination range (so e.g.
/// ``@intFromFloat(1.0e308)`` traps under wasi-libc); this helper
/// does the magnitude check explicitly so callers in the obj-
/// widening path stay safe.  Values that exceed i128 saturate at the
/// i128 boundary — callers that need full precision must go through
/// ``tcl_arith.float_to_bignum_obj`` (which uses Managed.setString).
fn float_to_i128_clamped(fval: f64) i128 {
    if (std.math.isNan(fval)) return 0;
    const i128_max_f: f64 = @floatFromInt(std.math.maxInt(i128));
    const i128_min_f: f64 = @floatFromInt(std.math.minInt(i128));
    if (fval >= i128_max_f) return std.math.maxInt(i128);
    if (fval <= i128_min_f) return std.math.minInt(i128);
    return @intFromFloat(fval);
}

/// i64 counterpart to :func:`float_to_i128_clamped` — clamps to the
/// i64 range and returns 0 for NaN.  Used by ``obj_get_int`` paths
/// that surface a float operand to a caller expecting an i64; debug
/// builds previously panicked on ``int(1.0e30)`` because the
/// unchecked ``@intFromFloat`` was outside the i64 range.
fn float_to_i64_clamped(fval: f64) i64 {
    if (std.math.isNan(fval)) return 0;
    const i64_max_f: f64 = @floatFromInt(std.math.maxInt(i64));
    const i64_min_f: f64 = @floatFromInt(std.math.minInt(i64));
    if (fval >= i64_max_f) return std.math.maxInt(i64);
    if (fval <= i64_min_f) return std.math.minInt(i64);
    return @intFromFloat(fval);
}

/// Promote any int-shaped operand to a freshly-allocated BigInt,
/// or return the existing borrowed BigInt for TYPE_BIGNUM.  The
/// boolean return tells the caller whether they own the result and
/// must :func:`bignum.destroy` it.
///
/// Used by arithmetic helpers in ``tcl_arith.zig`` when an
/// operation needs a ``*BigInt`` and any of its inputs may still
/// be in a smaller representation (TYPE_INT, immediate, or a
/// string literal that hasn't been promoted yet).
pub fn obj_promote_to_bignum(obj: i32) struct { m: ?*bignum.BigInt, owned: bool } {
    // Already a bignum — borrow.
    if (obj_get_bignum_managed(obj)) |m| return .{ .m = m, .owned = false };
    // String literal that exceeds i64 — parse fresh as bignum.
    if (obj != 0 and !is_immediate(obj)) {
        const addr: u32 = @bitCast(obj);
        const tag = read_i32(addr + OBJ_TYPE_TAG);
        if (tag == TYPE_STRING or tag == TYPE_INLINE_STRING) {
            const sptr: u32 = @bitCast(read_i32(addr + OBJ_STR_PTR));
            const slen: u32 = @bitCast(read_i32(addr + OBJ_STR_LEN));
            if (try_parse_int(sptr, slen) == null) {
                if (bignum.alloc_from_string(sptr, slen)) |m| {
                    return .{ .m = m, .owned = true };
                }
            }
        }
    }
    // Anything else collapses through the i128 path.
    const v = obj_get_bignum(obj);
    return .{ .m = bignum.alloc_from_int(v), .owned = true };
}

pub export fn obj_get_float(obj: i32) f64 {
    if (obj == 0) return 0.0;
    // S6.4 — tagged immediate: convert int → float directly.
    if (is_immediate(obj)) return @floatFromInt(immediate_unbox(obj));
    const addr: u32 = @bitCast(obj);
    const tag = read_i32(addr + OBJ_TYPE_TAG);
    if (tag == TYPE_FLOAT) return @bitCast(read_i64(addr + OBJ_INT_CACHE));
    if (tag == TYPE_INT) return @floatFromInt(read_i64(addr + OBJ_INT_CACHE));
    if (tag == TYPE_BIGNUM) {
        const ptr_addr: u32 = @bitCast(read_i32(addr + OBJ_DICT_EXT));
        if (ptr_addr == 0) return 0.0;
        const m: *bignum.BigInt = @ptrFromInt(ptr_addr);
        return bignum.to_f64(m);
    }
    if (tag == TYPE_STRING) {
        const sptr: u32 = @bitCast(read_i32(addr + OBJ_STR_PTR));
        const slen: u32 = @bitCast(read_i32(addr + OBJ_STR_LEN));
        if (try_parse_float(sptr, slen)) |val| return val;
        if (try_parse_int(sptr, slen)) |val| return @floatFromInt(val);
        if (bignum.parse_i128(sptr, slen)) |val| return @floatFromInt(val);
    }
    // S6.2 — inline string parses the inline payload through the
    // obj-internal pointer in OBJ_STR_PTR.
    if (tag == TYPE_INLINE_STRING) {
        const sptr: u32 = @bitCast(read_i32(addr + OBJ_STR_PTR));
        const slen: u32 = @bitCast(read_i32(addr + OBJ_STR_LEN));
        if (try_parse_float(sptr, slen)) |val| return val;
        if (try_parse_int(sptr, slen)) |val| return @floatFromInt(val);
        if (bignum.parse_i128(sptr, slen)) |val| return @floatFromInt(val);
    }
    return 0.0;
}

pub export fn obj_get_int(obj: i32) i64 {
    // Null/zero pointer sentinel — return 0 rather than reading
    // arbitrary bytes from address 0 (the pre-heap WASM stack area).
    // ``global_get`` returns 0 for unset variables; callers that pass
    // that result here expect to get the empty-string / zero integer.
    if (obj == 0) return 0;
    // S6.4 — tagged immediate: extract the value directly.
    if (is_immediate(obj)) return immediate_unbox(obj);
    const addr: u32 = @bitCast(obj);
    const tag = read_i32(addr + OBJ_TYPE_TAG);
    if (tag == TYPE_INT) return read_i64(addr + OBJ_INT_CACHE);
    if (tag == TYPE_FLOAT) {
        const fval: f64 = @bitCast(read_i64(addr + OBJ_INT_CACHE));
        return float_to_i64_clamped(fval);
    }
    if (tag == TYPE_BIGNUM) {
        // Truncating int conversion: take the low 64 bits of the
        // arbitrary-precision payload.  Matches C Tcl 9.0 behaviour
        // when an i64-only API (Tcl_GetWideIntFromObj) is asked for
        // a value that doesn't fit — caller must check overflow
        // before requesting the i64 form when accuracy is required.
        const ptr_addr: u32 = @bitCast(read_i32(addr + OBJ_DICT_EXT));
        if (ptr_addr == 0) return 0;
        const m: *bignum.BigInt = @ptrFromInt(ptr_addr);
        return bignum.to_i64(m);
    }
    if (tag == TYPE_STRING) {
        const sptr: u32 = @bitCast(read_i32(addr + OBJ_STR_PTR));
        const slen: u32 = @bitCast(read_i32(addr + OBJ_STR_LEN));
        if (try_parse_int(sptr, slen)) |val| {
            write_i32(addr + OBJ_TYPE_TAG, TYPE_INT);
            write_i64(addr + OBJ_INT_CACHE, val);
            return val;
        }
        // Float string: parse and truncate to integer (Tcl semantics:
        // ``expr {int("2.7")}`` = 2).  Do not cache as TYPE_INT since
        // the value retains its fractional form.
        if (try_parse_float(sptr, slen)) |fval| {
            return float_to_i64_clamped(fval);
        }
        // Bignum-string fallback: the literal exceeds i64 but fits
        // in i128.  We can't shimmer the obj into TYPE_BIGNUM here
        // (that would also require allocating the off-heap payload
        // and we're inside a read accessor) so just truncate to the
        // low 64 bits.  Callers that need the full value go through
        // :func:`obj_get_bignum`.
        if (bignum.parse_i128(sptr, slen)) |val| {
            return @as(i64, @truncate(val));
        }
        // Tcl boolean literals — ``true`` / ``yes`` / ``on`` → 1,
        // ``false`` / ``no`` / ``off`` → 0 (case-insensitive, and
        // Tcl also accepts unique prefixes).  Used by ``expr`` in
        // boolean contexts (``$x && $x`` with ``$x = "true"``) and
        // by tcltest's ``AcceptBoolean``.
        if (try_parse_bool(sptr, slen)) |val| {
            write_i32(addr + OBJ_TYPE_TAG, TYPE_INT);
            write_i64(addr + OBJ_INT_CACHE, val);
            return val;
        }
    }
    // S6.2 — inline string: parse the inline payload reachable
    // through the (obj-internal) pointer in OBJ_STR_PTR.  We
    // don't shimmer-cache the parsed value because that would
    // clobber the string bytes the inline encoding shares the
    // OBJ_INT_CACHE slot with.
    if (tag == TYPE_INLINE_STRING) {
        const sptr: u32 = @bitCast(read_i32(addr + OBJ_STR_PTR));
        const slen: u32 = @bitCast(read_i32(addr + OBJ_STR_LEN));
        if (try_parse_int(sptr, slen)) |val| return val;
        if (try_parse_float(sptr, slen)) |fval| return float_to_i64_clamped(fval);
        if (try_parse_bool(sptr, slen)) |val| return val;
        return 0;
    }
    return read_i64(addr + OBJ_INT_CACHE);
}

/// Recognise Tcl boolean keywords (case-insensitive) — ``true`` /
/// ``yes`` / ``on`` → 1, ``false`` / ``no`` / ``off`` → 0.  Returns
/// ``null`` if the string is not a recognised boolean.  Matches the
/// set accepted by ``Tcl_GetBooleanFromObj``.
pub fn try_parse_bool(ptr: u32, len: u32) ?i64 {
    if (len == 0 or len > 5) return null;
    const src: [*]const u8 = @ptrFromInt(ptr);
    var buf: [5]u8 = undefined;
    for (0..len) |i| {
        const c = src[i];
        buf[i] = if (c >= 'A' and c <= 'Z') c + 32 else c;
    }
    const lc = buf[0..len];
    // True-valued keywords.
    if (std_eq(lc, "true") or std_eq(lc, "yes") or std_eq(lc, "on") or
        std_eq(lc, "tru") or std_eq(lc, "tr") or std_eq(lc, "t") or
        std_eq(lc, "ye") or std_eq(lc, "y"))
        return 1;
    // False-valued keywords (``off`` / ``of`` need the longer match
    // before ``of`` so ``on``/``of`` don't collide; handled by the
    // exact-match comparisons above already).
    if (std_eq(lc, "false") or std_eq(lc, "fals") or std_eq(lc, "fal") or
        std_eq(lc, "fa") or std_eq(lc, "f") or
        std_eq(lc, "no") or std_eq(lc, "n") or
        std_eq(lc, "off") or std_eq(lc, "of"))
        return 0;
    return null;
}

const std_eq = chars.slice_eq;

pub fn try_parse_int(ptr: u32, len: u32) ?i64 {
    if (len == 0) return null;
    const src: [*]const u8 = @ptrFromInt(ptr);
    var i: u32 = 0;
    while (i < len and is_space(src[i])) i += 1;
    if (i >= len) return null;
    var negative = false;
    if (src[i] == '-') {
        negative = true;
        i += 1;
    } else if (src[i] == '+') {
        i += 1;
    }
    if (i >= len) return null;
    if (src[i] < '0' or src[i] > '9') return null;
    // Accumulate magnitude in u64 so the i64::MIN literal
    // ``-9223372036854775808`` parses without panicking — its
    // absolute value is one above i64::MAX, which an i64 accumulator
    // would trip on the final ``val * 10 + 8`` step under Zig's
    // debug-mode overflow checks.  Overflow yields ``null`` so
    // callers can fall back through the float / string path.
    var mag: u64 = 0;
    while (i < len and src[i] >= '0' and src[i] <= '9') {
        const m = @mulWithOverflow(mag, @as(u64, 10));
        if (m[1] != 0) return null;
        const a = @addWithOverflow(m[0], @as(u64, src[i] - '0'));
        if (a[1] != 0) return null;
        mag = a[0];
        i += 1;
    }
    while (i < len and is_space(src[i])) i += 1;
    if (i != len) return null;
    const I64_MIN_ABS: u64 = 9223372036854775808;
    const I64_MAX_U: u64 = 9223372036854775807;
    if (negative) {
        if (mag > I64_MIN_ABS) return null;
        if (mag == I64_MIN_ABS) return @as(i64, -9223372036854775807) - 1;
        return -@as(i64, @intCast(mag));
    }
    if (mag > I64_MAX_U) return null;
    return @as(i64, @intCast(mag));
}

/// Parse a decimal float literal (e.g. "3.14", "2.2e5", "-0.5").
/// Returns null if the string is not a valid float or is a plain integer.
/// Requires a decimal point or exponent to distinguish from integers.
pub fn try_parse_float(ptr: u32, len: u32) ?f64 {
    if (len == 0) return null;
    const src: [*]const u8 = @ptrFromInt(ptr);
    var i: u32 = 0;
    while (i < len and is_space(src[i])) i += 1;
    if (i >= len) return null;
    const start = i;
    if (src[i] == '-' or src[i] == '+') i += 1;
    if (i >= len) return null;
    var has_dot = false;
    var has_exp = false;
    var has_digit = false;
    while (i < len) {
        const c = src[i];
        if (c >= '0' and c <= '9') {
            has_digit = true;
            i += 1;
        } else if (c == '.' and !has_dot and !has_exp) {
            has_dot = true;
            i += 1;
        } else if ((c == 'e' or c == 'E') and !has_exp and has_digit) {
            has_exp = true;
            i += 1;
            if (i < len and (src[i] == '+' or src[i] == '-')) i += 1;
        } else break;
    }
    while (i < len and is_space(src[i])) i += 1;
    if (i != len) return null;
    if (!has_digit) return null;
    if (!has_dot and !has_exp) return null; // plain integer — not a float
    // Copy the non-whitespace slice to a stack buffer and parse.
    if (len > 64) return null;
    _ = start;
    // Find end of non-whitespace content.
    var end = len;
    while (end > 0 and is_space(src[end - 1])) end -= 1;
    // Find start of non-whitespace.
    var beg: u32 = 0;
    while (beg < end and is_space(src[beg])) beg += 1;
    if (beg >= end) return null;
    var buf: [65]u8 = undefined;
    const blen = end - beg;
    for (0..blen) |k| buf[k] = src[beg + k];
    return std.fmt.parseFloat(f64, buf[0..blen]) catch null;
}

pub export fn tcl_obj_retain(obj: i32) void {
    // Null-safe to mirror ``tcl_obj_release``.  Compile-side
    // call sites (frame-elided proc prologues, the future
    // owned-slot retain wrap from S2.2) emit the retain
    // unconditionally on every param / value to keep the call
    // sequence regular; without this guard the very first param
    // slot (which is 0 at function entry when the caller passed
    // no argument) would write to address 0 in linear memory.
    if (obj == 0) return;
    // S6.4 — tagged immediates have no refcount header (immortal).
    if (is_immediate(obj)) return;
    const addr: u32 = @bitCast(obj);
    // Use-after-free guard: same poison + sanity checks as
    // ``tcl_obj_release``.  Retaining a slab that's currently on a
    // per-class free-list would write ``previous_head + 1`` to the
    // slab's first word, corrupting the next-link.  See the
    // discussion on ``tcl_obj_release`` above.
    const tag = read_i32(addr + OBJ_TYPE_TAG);
    if (tag == TYPE_FREED_POISON) {
        if (build_options.leak_check) g_double_free_count += 1;
        return;
    }
    const rc = read_i32(addr + OBJ_REFCOUNT);
    if (rc < 0 or rc > (1 << 20)) {
        if (build_options.leak_check) g_double_free_count += 1;
        return;
    }
    // PR #237 second-pass review: refuse to retain a deferred-free-
    // queued obj (rc == 0 marker).  Without this guard, the sequence
    //
    //     release(A)  → rc 1→0, queue A
    //     retain(A)   → rc 0→1 (resurrects A on the queue)
    //     release(A)  → rc 1→0, queue A AGAIN
    //     drain       → free A twice → free-list double-entry
    //
    // gives a duplicated free-list slab that two future ``alloc``
    // calls receive as the same address.  The release path already
    // guards rc==0 (records double-release in leakcheck); make
    // retain symmetric so the resurrection-into-second-queue path
    // is impossible.  A retain after a defer is itself a refcount-
    // discipline violation; record it for leakcheck visibility.
    if (rc == 0) {
        if (build_options.leak_check) g_double_free_count += 1;
        return;
    }
    write_i32(addr + OBJ_REFCOUNT, rc + 1);
}

// Deferred-free queue.  Holds TclObj headers whose refcount
// reached zero but whose buffer/header haven't been pushed onto
// the free-lists yet.  We drain at safe points (between
// statements in the eval loop) so a ``(ptr, len)`` borrowed from
// a soon-to-be-released TclObj cannot alias a freshly-recycled
// allocation.
//
// The queue is a fixed-size ring (drains often enough that
// overflow is unlikely on real workloads).  On overflow we bypass
// the deferral and free immediately — degrades to the old aliasing
// risk but doesn't lose memory.
// Large queue so MM-B.4 (release every parser word per statement)
// doesn't overflow into immediate-free, which would race with
// libc malloc re-issuing a slab while it's still borrowed by an
// outer eval_script's parser walk.
const PENDING_FREE_CAP: u32 = 65536;
var pending_free: [PENDING_FREE_CAP]u32 = [_]u32{0} ** PENDING_FREE_CAP;
var pending_free_count: u32 = 0;

fn release_now(addr: u32) void {
    if (build_options.leak_check) g_alloc_count -= 1;
    // Free any dict hash side-cache OR bignum payload before the obj
    // header is freed — both share the OBJ_DICT_EXT slot (a single
    // TclObj is one type at a time, so the slot's owning subsystem
    // is determined by ``OBJ_TYPE_TAG``).
    //
    // Done first because ``dict_destroy_ext`` may trigger a chain
    // of releases that recursively land in ``release_now`` for the
    // value handles, and the obj header bytes must still be valid
    // (not freed) for those recursive releases to read the
    // ``OBJ_DICT_EXT`` field correctly during their own cleanup.
    const tag = read_i32(addr + OBJ_TYPE_TAG);
    const ext: u32 = @bitCast(read_i32(addr + OBJ_DICT_EXT));
    if (ext != 0) {
        write_i32(addr + OBJ_DICT_EXT, 0);
        if (tag == TYPE_BIGNUM) {
            const m: *bignum.BigInt = @ptrFromInt(ext);
            bignum.destroy(m);
        } else {
            const tcl_dict = @import("tcl_dict.zig");
            tcl_dict.dict_destroy_ext(ext);
        }
    }
    const cap: u32 = @bitCast(read_i32(addr + OBJ_STR_CAP));
    if (cap > 0) {
        const sp: u32 = @bitCast(read_i32(addr + OBJ_STR_PTR));
        if (sp != 0) {
            // MM-B.6: invalidate any parse_cache entry keyed on
            // this buffer before returning it to libc malloc.
            // Without this, a subsequent ``alloc`` could re-issue
            // the slab to a new TclObj whose ``(body_ptr,
            // body_len)`` happens to match a stale entry, and
            // ``parse_cache.lookup`` would return tokens that
            // point into freed memory — observed as ``unknown
            // command: <binary garbage>`` traps at site=146 in
            // cmdIL.test.
            const parse_cache = @import("parse_cache.zig");
            parse_cache.invalidate_for_buffer(sp);
            free_sized(sp, cap);
        }
    }
    free_obj(addr);
}

/// Drain the deferred-free queue.  Called by the eval loop between
/// statements so all references to a since-released TclObj's bytes
/// have been consumed before the slab is reissued.
///
/// MM-B race fix: ``tcl_obj_release`` queues by zeroing the obj's
/// refcount field as a marker.  If the slab gets recycled by an
/// ``obj_alloc`` between the release and the drain, the new tenant
/// resets the refcount to 1 — drain detects that here and skips
/// the free, leaving the new tenant alone.  Without this guard
/// the drain would free the new tenant's buffer (use-after-free
/// observable as ``NONE`` trap messages and other "value
/// vanished" symptoms in long-running tcltest workloads).
pub export fn tcl_obj_drain_pending() void {
    var i: u32 = 0;
    while (i < pending_free_count) : (i += 1) {
        const addr = pending_free[i];
        const rc = read_i32(addr + OBJ_REFCOUNT);
        if (rc != 0) continue; // slab was reissued — leave alone
        release_now(addr);
    }
    pending_free_count = 0;
}

pub export fn tcl_obj_release(obj: i32) void {
    if (obj == 0) return;
    // S6.4 — tagged immediates have no refcount and no buffer
    // to free; release is a no-op.
    if (is_immediate(obj)) return;
    const addr: u32 = @bitCast(obj);
    // Use-after-free guard #1: poison check.  ``free_obj`` writes
    // ``TYPE_FREED_POISON`` into the slab's type tag before recycling
    // it.  ``obj_alloc`` writes ``TYPE_STRING`` over the marker when
    // the slab is reissued, so the marker is only present while the
    // slab sits on a free-list awaiting a new tenant.  A stale handle
    // that observes the marker is releasing a slab that no longer
    // backs any logical TclObj — skip the decrement-and-write so
    // the freelist next-link (overlapping ``OBJ_REFCOUNT``) stays
    // intact.
    const tag = read_i32(addr + OBJ_TYPE_TAG);
    if (tag == TYPE_FREED_POISON) {
        if (build_options.leak_check) g_double_free_count += 1;
        return;
    }
    const rc = read_i32(addr + OBJ_REFCOUNT);
    if (rc == 0) {
        // Already on the deferred-free queue (rc==0 marker) or
        // freed.  Any further release is the over-release pattern
        // S2's failed attempt hit; record it so leakcheck builds
        // surface it.  Production builds compile this branch out.
        if (build_options.leak_check) g_double_free_count += 1;
        return;
    }
    // Use-after-free guard #2: refcount sanity check.  A live obj's
    // refcount tops out at a few dozen in real workloads.  An
    // implausibly large value means we're reading a freelist
    // next-link (or other heap address) out of a freed slab's first
    // word — typically when a stale handle survives across a free.
    // Skip the decrement so the freelist chain stays intact instead
    // of accumulating a "head - 1" corruption that traps later in
    // ``alloc``.  Belt-and-braces with the poison check above; one
    // catches the slab-on-freelist case, the other catches an
    // already-libc-freed slab whose first word happens to hold any
    // garbage.
    if (rc < 0 or rc > (1 << 20)) {
        if (build_options.leak_check) g_double_free_count += 1;
        return;
    }
    if (rc <= 1) {
        // Defer the actual free so a ``(ptr, len)`` reference
        // borrowed elsewhere can't alias a freshly-reissued slab.
        // ``OBJ_STR_CAP`` is non-zero only for buffers we allocated
        // ourselves; zero means the str_ptr points into a wasm data
        // segment / interned literal we must not free.
        if (pending_free_count < PENDING_FREE_CAP) {
            // Mark the obj as "in-flight free" by zeroing the
            // refcount so a stray re-release on the same handle
            // becomes a no-op (we already counted it).
            write_i32(addr + OBJ_REFCOUNT, 0);
            pending_free[pending_free_count] = addr;
            pending_free_count += 1;
        } else {
            // Queue full — fall back to immediate free.  Acceptable
            // worst case: drains happen often enough that this is
            // rare in practice.
            release_now(addr);
        }
    } else {
        write_i32(addr + OBJ_REFCOUNT, rc - 1);
    }
}

pub export fn tcl_var_set(value: i32) i32 {
    return value;
}

pub export fn tcl_var_get(value: i32) i32 {
    return value;
}

// -- TclObj string helpers --

pub fn obj_str_ptr(obj: i32) u32 {
    // S6.4 — tagged immediates have no string buffer; callers
    // expecting a buffer need ``obj_ensure_string`` first.
    if (is_immediate(obj)) return 0;
    const addr: u32 = @bitCast(obj);
    // S6.2 — inline strings populate ``OBJ_STR_PTR`` with the
    // inline buffer's address (= obj + OBJ_INT_CACHE), so this
    // path needs no special-casing.
    return @bitCast(read_i32(addr + OBJ_STR_PTR));
}

pub fn obj_str_len(obj: i32) u32 {
    if (is_immediate(obj)) return 0;
    const addr: u32 = @bitCast(obj);
    return @bitCast(read_i32(addr + OBJ_STR_LEN));
}

pub fn obj_type(obj: i32) i32 {
    // S6.4 — tagged immediates always represent integers.
    if (is_immediate(obj)) return TYPE_INT;
    const addr: u32 = @bitCast(obj);
    const tag = read_i32(addr + OBJ_TYPE_TAG);
    // S6.2 — inline-string encoding is a representational detail;
    // callers checking ``obj_type`` for ``TYPE_STRING`` shouldn't
    // have to know about it.
    if (tag == TYPE_INLINE_STRING) return TYPE_STRING;
    return tag;
}

/// Copy *len* bytes from src to dst in linear memory.  Null-safe when
/// *len* is 0 — callers like the hash-table insert path may pass
/// ``(dst=0, src=0, len=0)`` when storing an empty-string key (an
/// alias with a stale target descriptor for instance), and
/// ``@ptrFromInt(0)`` would panic in debug builds even though the
/// loop body never runs.
pub fn memcpy(dst: u32, src: u32, len: u32) void {
    if (len == 0 or src == 0 or dst == 0) return;
    const d: [*]u8 = @ptrFromInt(dst);
    const s: [*]const u8 = @ptrFromInt(src);
    for (0..len) |i| {
        d[i] = s[i];
    }
}

/// Create a new string TclObj by copying *len* bytes from *src*.
/// The new TclObj owns its byte buffer (capacity = aligned alloc
/// size) so subsequent appends can grow in place when refcount==1.
pub fn obj_new_string_copy(src: u32, len: u32) i32 {
    if (len == 0) return obj_new_string(0, 0);
    // S6.2 — inline-string optimisation.  Strings short enough to
    // fit in the obj header skip the second ``alloc`` call; the
    // bytes live directly at ``OBJ_INT_CACHE`` and ``str_ptr`` is
    // 0 to flag the inline encoding.  ``str_cap`` stays 0 so
    // ``release_now`` doesn't try to free a non-existent buffer.
    if (len <= MAX_INLINE_STR) {
        const obj = obj_alloc();
        write_i32(obj + OBJ_TYPE_TAG, TYPE_INLINE_STRING);
        memcpy(obj + OBJ_INT_CACHE, src, len);
        // Point str_ptr at the inline buffer so callers reading
        // OBJ_STR_PTR / OBJ_STR_LEN directly (e.g. test harnesses
        // and dispatch helpers that don't go through
        // ``obj_str_ptr``) see a valid pointer transparently.
        // ``str_cap`` stays 0 so ``release_now`` doesn't try to
        // ``free`` an address that's part of the obj header.
        // ``@bitCast`` rather than ``@intCast``: ``obj`` is a u32
        // allocator address that we round-trip into the i32 obj
        // handle slot.  Above 2 GiB the signed-narrow form panics in
        // debug builds — see issue #303.
        write_i32(obj + OBJ_STR_PTR, @bitCast(obj + OBJ_INT_CACHE));
        write_i32(obj + OBJ_STR_LEN, @bitCast(len));
        return @bitCast(obj);
    }
    // Round capacity up to the smallest size class so the recycler
    // can return the slab to the right free-list at end-of-life.
    const cap = round_up_to_class((len + 7) & ~@as(u32, 7));
    const buf = alloc(cap);
    if (buf == 0) return 0;
    memcpy(buf, src, len);
    const obj = obj_new_string(@bitCast(buf), @bitCast(len));
    if (obj != 0) {
        write_i32(@as(u32, @bitCast(obj)) + OBJ_STR_CAP, @bitCast(cap));
    }
    return obj;
}

/// Wrap an already-``alloc``-ed buffer in a fresh string TclObj that
/// *owns* the buffer — ``release_now`` will hand the slab back via
/// ``free_sized`` instead of leaking it.  The plain ``obj_new_string``
/// stores ``cap = 0`` (the borrowing variant for buffers that live in
/// the wasm data segment or in another obj's storage); callers that
/// allocated a fresh ``buf`` and want the obj to claim ownership must
/// go through this helper or set ``OBJ_STR_CAP`` themselves.
///
/// ``alloc_size`` is the original argument passed to ``alloc``; the
/// stored cap is rounded symmetrically with ``alloc``'s own rounding so
/// ``free_sized`` recycles the slab to the right free-list.
pub fn obj_new_string_take(buf: u32, len: u32, alloc_size: u32) i32 {
    // Compute the symmetric size class up-front so the OOM cleanup
    // path can hand ``buf`` back to ``free_sized`` with the same
    // class the caller's ``alloc`` requested.  Without this the
    // OOM exit (obj header alloc fails after we already own
    // ``buf``) would leak the caller-provided buffer — defeating
    // the whole point of the take-ownership helper (Copilot review
    // on PR #322).
    const aligned = (alloc_size + 7) & ~@as(u32, 7);
    const cap = if (aligned <= 2048) round_up_to_class(aligned) else aligned;
    const obj = obj_new_string(@bitCast(buf), @bitCast(len));
    if (obj == 0) {
        if (buf != 0 and alloc_size != 0) free_sized(buf, cap);
        return 0;
    }
    if (buf == 0 or alloc_size == 0) return obj;
    write_i32(@as(u32, @bitCast(obj)) + OBJ_STR_CAP, @bitCast(cap));
    return obj;
}

// Scratch buffer for integer-to-string conversion (no newline)
var itoa_buf: [21]u8 = undefined;

// S6.4 — Scratch ring for ``obj_ensure_string`` of tagged
// immediates.  Tagged immediates have no header to cache the
// rendered string into, so each call to ``obj_ensure_string``
// must produce a fresh-looking buffer.  The single ``itoa_buf``
// would be overwritten by any nested rendering (consider
// ``format "%d-%d" $a $b`` where both args are immediates), so
// we rotate through a ring.
//
// **PR #237 review**: previously ``IMMEDIATE_RING_SIZE = 8`` with
// no overflow protection — a caller producing 9+ simultaneously-
// live renders silently corrupted earlier outputs by wrapping
// back into in-use slots.  The size has been bumped to 32 to
// comfortably exceed every current caller's deepest nesting
// (``format`` with up to 16 ``%d`` substitutions, ``lappend``
// chains, ``regsub`` capture interpolation).  The ring's running
// total is exposed via ``tcl_immediate_ring_total_renders`` so
// tests can detect a regression in nesting headroom.
//
// Production builds keep the silent wrap (correctness still
// degrades to "earliest renders may be stale" rather than to
// random memory corruption — the ring storage is module-static
// and bounded), with much more generous headroom than before.
const IMMEDIATE_RING_SIZE: u32 = 32;
const IMMEDIATE_BUF_BYTES: u32 = 24;
var immediate_string_ring: [IMMEDIATE_RING_SIZE][IMMEDIATE_BUF_BYTES]u8 = undefined;
var immediate_ring_cursor: u32 = 0;
pub var immediate_ring_total_renders: u64 = 0;
pub export fn tcl_immediate_ring_total_renders() i64 {
    return @bitCast(immediate_ring_total_renders);
}

pub fn itoa(value: i64) struct { ptr: [*]u8, len: u32 } {
    // Compute the magnitude in u64 so the ``-i64::MIN`` corner case
    // doesn't panic — the i64 ``-v`` operator under Zig's debug
    // overflow checks would trip here, since ``-i64::MIN`` exceeds
    // i64::MAX by 1.  Rendering through a u64 magnitude avoids that.
    var negative = false;
    var mag: u64 = blk: {
        if (value < 0) {
            negative = true;
            // Bitcast trick: ``0 -% v`` in u64 yields the magnitude
            // for any negative i64, including i64::MIN whose
            // absolute value is 2^63.
            const u: u64 = @bitCast(value);
            break :blk @as(u64, 0) -% u;
        }
        break :blk @as(u64, @intCast(value));
    };
    var i: u32 = itoa_buf.len - 1;
    if (mag == 0) {
        itoa_buf[i] = '0';
    } else {
        while (mag > 0) {
            itoa_buf[i] = @as(u8, @intCast(mag % 10)) + '0';
            mag /= 10;
            if (mag > 0) i -= 1;
        }
    }
    if (negative) {
        i -= 1;
        itoa_buf[i] = '-';
    }
    return .{ .ptr = @as([*]u8, &itoa_buf) + i, .len = itoa_buf.len - i };
}

// Scratch buffer for float-to-string conversion (max 32 bytes).
var ftoa_buf: [32]u8 = undefined;

fn ftoa(value: f64) struct { ptr: [*]u8, len: u32 } {
    // Tcl 9 renders non-finite floats as ``Inf``, ``-Inf``, ``NaN``
    // — not Zig's lowercase ``inf`` / ``nan``.  Special-case before
    // the generic decimal path so the trailing ``.0`` fixup below
    // doesn't append to ``inf`` and produce ``inf.0``.
    if (std.math.isNan(value)) {
        const lit = "NaN";
        @memcpy(ftoa_buf[0..lit.len], lit);
        return .{ .ptr = ftoa_buf[0..].ptr, .len = @intCast(lit.len) };
    }
    if (std.math.isInf(value)) {
        if (value < 0) {
            const lit = "-Inf";
            @memcpy(ftoa_buf[0..lit.len], lit);
            return .{ .ptr = ftoa_buf[0..].ptr, .len = @intCast(lit.len) };
        }
        const lit = "Inf";
        @memcpy(ftoa_buf[0..lit.len], lit);
        return .{ .ptr = ftoa_buf[0..].ptr, .len = @intCast(lit.len) };
    }
    const result = std.fmt.bufPrint(&ftoa_buf, "{d}", .{value}) catch ftoa_buf[0..1];
    const len = result.len;
    // Tcl requires floats to look like floats: ensure the string contains
    // a '.', 'e', or 'E' so that "5.0" is not confused with integer "5".
    var has_dot = false;
    for (result) |c| {
        if (c == '.' or c == 'e' or c == 'E') {
            has_dot = true;
            break;
        }
    }
    if (!has_dot and len + 2 <= ftoa_buf.len) {
        ftoa_buf[len] = '.';
        ftoa_buf[len + 1] = '0';
        return .{ .ptr = ftoa_buf[0..].ptr, .len = @intCast(len + 2) };
    }
    return .{ .ptr = ftoa_buf[0..].ptr, .len = @intCast(len) };
}

/// Render a TclObj to its string representation (integer, float, or string).
pub fn obj_ensure_string(obj: i32) struct { ptr: u32, len: u32 } {
    if (obj == 0) return .{ .ptr = 0, .len = 0 };
    // S6.4 — tagged immediate: render into the next ring buffer.
    // Rotating buffers means consecutive renders of distinct
    // immediates (e.g. ``format "%d-%d" $a $b``) keep their
    // results valid until the ring wraps.
    if (is_immediate(obj)) {
        const result = itoa(immediate_unbox(obj));
        const slot = immediate_ring_cursor;
        immediate_ring_cursor = (immediate_ring_cursor + 1) % IMMEDIATE_RING_SIZE;
        immediate_ring_total_renders += 1;
        const dst_ptr: u32 = @intFromPtr(&immediate_string_ring[slot]);
        memcpy(dst_ptr, @intFromPtr(result.ptr), result.len);
        return .{ .ptr = dst_ptr, .len = result.len };
    }
    const addr: u32 = @bitCast(obj);
    const tag = read_i32(addr + OBJ_TYPE_TAG);
    if (tag == TYPE_STRING) {
        return .{
            .ptr = @bitCast(read_i32(addr + OBJ_STR_PTR)),
            .len = @bitCast(read_i32(addr + OBJ_STR_LEN)),
        };
    }
    // S6.2 — inline strings populate OBJ_STR_PTR with the inline
    // buffer's address.  The pointer aliases the obj header
    // itself; callers must not hold it past the obj's lifetime.
    if (tag == TYPE_INLINE_STRING) {
        return .{
            .ptr = @bitCast(read_i32(addr + OBJ_STR_PTR)),
            .len = @bitCast(read_i32(addr + OBJ_STR_LEN)),
        };
    }
    const sptr: u32 = @bitCast(read_i32(addr + OBJ_STR_PTR));
    if (sptr != 0) {
        return .{
            .ptr = sptr,
            .len = @bitCast(read_i32(addr + OBJ_STR_LEN)),
        };
    }
    if (tag == TYPE_FLOAT) {
        const fval: f64 = @bitCast(read_i64(addr + OBJ_INT_CACHE));
        const result = ftoa(fval);
        // PR #237 review: also set ``OBJ_STR_CAP`` so the buffer
        // can be reclaimed by ``release_now`` (which gates the
        // free on ``cap > 0``).  Use the same size-class rounding
        // the recycler expects on the free side — the free path
        // matches ``alloc(cap)`` only when ``cap`` is the class-
        // rounded size.  Without recording the cap, the rendered
        // buffer leaks once per TYPE_FLOAT obj that's ever been
        // stringified.
        const cap = round_up_to_class((result.len + 7) & ~@as(u32, 7));
        const buf = alloc(cap);
        if (buf == 0) return .{ .ptr = 0, .len = 0 };
        memcpy(buf, @intFromPtr(result.ptr), result.len);
        write_i32(addr + OBJ_STR_PTR, @bitCast(buf));
        write_i32(addr + OBJ_STR_LEN, @bitCast(result.len));
        write_i32(addr + OBJ_STR_CAP, @bitCast(cap));
        return .{ .ptr = buf, .len = result.len };
    }
    if (tag == TYPE_BIGNUM) {
        const ptr_addr: u32 = @bitCast(read_i32(addr + OBJ_DICT_EXT));
        if (ptr_addr == 0) return .{ .ptr = 0, .len = 0 };
        const m: *bignum.BigInt = @ptrFromInt(ptr_addr);
        const rendered = bignum.alloc_format(m) orelse return .{ .ptr = 0, .len = 0 };
        defer bignum.allocator.free(rendered);
        const len: u32 = @intCast(rendered.len);
        const cap = round_up_to_class((len + 7) & ~@as(u32, 7));
        const buf = alloc(cap);
        if (buf == 0) return .{ .ptr = 0, .len = 0 };
        memcpy(buf, @intFromPtr(rendered.ptr), len);
        write_i32(addr + OBJ_STR_PTR, @bitCast(buf));
        write_i32(addr + OBJ_STR_LEN, @bitCast(len));
        write_i32(addr + OBJ_STR_CAP, @bitCast(cap));
        return .{ .ptr = buf, .len = len };
    }
    const val = read_i64(addr + OBJ_INT_CACHE);
    const result = itoa(val);
    // Same OBJ_STR_CAP discipline as the float path — see comment
    // above.  Pre-fix, the rendered buffer leaked once per
    // TYPE_INT obj that was ever stringified.
    const cap = round_up_to_class((result.len + 7) & ~@as(u32, 7));
    const buf = alloc(cap);
    if (buf == 0) return .{ .ptr = 0, .len = 0 };
    memcpy(buf, @intFromPtr(result.ptr), result.len);
    write_i32(addr + OBJ_STR_PTR, @bitCast(buf));
    write_i32(addr + OBJ_STR_LEN, @bitCast(result.len));
    write_i32(addr + OBJ_STR_CAP, @bitCast(cap));
    return .{ .ptr = buf, .len = result.len };
}

// Character classification re-exports: callers that already say
// ``obj.is_space`` / ``obj.str_cmp`` keep working, but new code should
// import ``tcl_chars.zig`` directly.
pub const is_space = chars.is_space;
pub const str_cmp = chars.str_cmp;

// Backslash-decoding and list-parsing helpers live in their own
// modules.  Re-exports here keep existing ``obj.encode_utf8`` /
// ``obj.consume_bs_escape`` / ``obj.list_count_elements`` /
// ``obj.list_element_at`` / ``obj.copy_unbraced_elem`` callers
// working during migration — new code should import ``tcl_bs.zig``
// or ``tcl_list_parse.zig`` directly.
const bs = @import("tcl_bs.zig");
const list_parse = @import("tcl_list_parse.zig");
pub const encode_utf8 = bs.encode_utf8;
pub const consume_bs_escape = bs.consume_bs_escape;
pub const list_count_elements = list_parse.count_elements;
pub const list_validate_braces = list_parse.validate_list_braces;
pub const copy_unbraced_elem = list_parse.copy_unbraced_elem;

/// Re-export of :func:`tcl_list_parse.element_at` with the legacy
/// anonymous-struct return type so existing ``obj.list_element_at``
/// callers don't have to switch to the named ``Element`` struct in
/// the same change as the file move.  New code should prefer the
/// named type.
pub fn list_element_at(ptr: u32, len: u32, idx: i64) struct { start: u32, len: u32, braced: bool } {
    const e = list_parse.element_at(ptr, len, idx);
    return .{ .start = e.start, .len = e.len, .braced = e.braced };
}

// List-element scan / convert / quote helpers live in
// ``tcl_list_quote.zig``.  Re-export surface area so existing callers
// that say ``obj.scan_element`` / ``obj.list_elem_quote`` etc. keep
// working — new code should import ``tcl_list_quote.zig`` directly.
const list_quote = @import("tcl_list_quote.zig");
pub const FLAG_CONVERT_NONE = list_quote.FLAG_CONVERT_NONE;
pub const FLAG_DONT_USE_BRACES = list_quote.FLAG_DONT_USE_BRACES;
pub const FLAG_CONVERT_BRACE = list_quote.FLAG_CONVERT_BRACE;
pub const FLAG_CONVERT_ESCAPE = list_quote.FLAG_CONVERT_ESCAPE;
pub const FLAG_DONT_QUOTE_HASH = list_quote.FLAG_DONT_QUOTE_HASH;
pub const FLAG_CONVERT_MASK = list_quote.FLAG_CONVERT_MASK;
pub const scan_element = list_quote.scan_element;
pub const convert_element = list_quote.convert_element;
pub const list_elem_quote = list_quote.list_elem_quote;
pub const list_elem_quote_nth = list_quote.list_elem_quote_nth;
