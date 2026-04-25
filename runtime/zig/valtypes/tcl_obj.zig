// TclObj value type, memory management, and string helpers for WASM.
//
// Memory layout of a TclObj:
//   offset 0: refcount  (i32)
//   offset 4: type_tag  (i32)  0=string, 1=int, 2=list
//   offset 8: int_cache (i64)  cached integer representation
//   offset 16: str_ptr  (i32)  pointer to UTF-8 data in linear memory
//   offset 20: str_len  (i32)  byte length of the string representation
//   Total: 24 bytes per TclObj
//
// Layering: character classification and byte-span comparison are in
// ``tcl_chars.zig``; list-quoting / list-parsing / backslash decoding
// live in their own modules.  This file owns the TclObj memory model
// and integer / boolean scalar parsers.  ``pub const`` re-exports of
// ``is_space`` / ``is_scan_space`` / ``str_cmp`` are kept as
// compatibility shims so existing ``obj.is_space`` callers work —
// new code should import ``tcl_chars.zig`` directly.

const chars = @import("tcl_chars.zig");
const std = @import("std");

// Type tags
pub const TYPE_STRING: i32 = 0;
pub const TYPE_INT: i32 = 1;
pub const TYPE_LIST: i32 = 2;
pub const TYPE_DICT: i32 = 3;
// TYPE_FLOAT: f64 bits stored in the int_cache field via @bitCast.
// The str_ptr/str_len fields hold a cached string representation
// once obj_ensure_string has been called.
pub const TYPE_FLOAT: i32 = 4;

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
pub const OBJ_SIZE: u32 = 32;

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

var heap_ptr: u32 = PAGE_SIZE;
var oom_flag: u32 = 0;

// Per-size-class free-lists.  ``free_lists[i]`` is the head of a
// singly-linked list of recycled slabs whose aligned size matches
// ``size_classes[i]``.  Each free slab stores the ``next`` pointer
// in its first 4 bytes (i.e. at ``addr``).
//
// Sizes are chosen to cover the common allocation shapes:
//   - 32 bytes   = OBJ_SIZE (every TclObj header — index 0)
//   - 48, 64, 96 = small string buffers, capacity-aware append starts
//   - 128, 192, 256, 384 = mid-sized list backings, dict pairs
//   - 512, 1024, 2048 = large list/string buffers, frame tables
// Anything larger than 2048 falls through to the bump path and is
// not recycled (rare in practice; capacity-aware append doubles
// past 2048 only on long-running text-builder workloads).
const SIZE_CLASSES = [_]u32{ 32, 48, 64, 96, 128, 192, 256, 384, 512, 1024, 2048 };
var free_lists = [_]u32{0} ** SIZE_CLASSES.len;

// Back-compat alias — pre-1.2 code referred to a single ``free_list``
// for OBJ_SIZE.  Anything that read the old name now gets the new
// class-0 head.
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

/// Ensure ``heap_ptr + size`` fits inside the current linear memory,
/// growing by enough whole pages to cover the request when not.
/// Returns ``true`` on success; ``false`` if the runtime cap is hit
/// or the wasm host refuses to grow.  Callers should treat ``false``
/// as a fatal allocation failure: ``alloc`` raises the OOM flag and
/// returns 0 so a caller that derefs the result lands in a deterministic
/// trap rather than a wild address.
fn ensure_capacity(needed_end: u32) bool {
    const current_pages: u32 = @intCast(@wasmMemorySize(0));
    const current_bytes: u32 = current_pages * PAGE_SIZE;
    if (needed_end <= current_bytes) return true;
    const want_bytes = needed_end - current_bytes;
    var want_pages = (want_bytes + PAGE_SIZE - 1) / PAGE_SIZE;
    // Grow geometrically — at minimum the requested page count, but
    // double the current size when that's larger.  Keeps amortised
    // grow cost O(1) per allocation across a long bundle while still
    // honouring small allocations exactly.
    if (want_pages < current_pages) want_pages = current_pages;
    if (current_pages + want_pages > MAX_HEAP_PAGES) {
        // Take whatever's left up to the cap.
        if (current_pages >= MAX_HEAP_PAGES) return false;
        want_pages = MAX_HEAP_PAGES - current_pages;
        if (current_bytes + want_pages * PAGE_SIZE < needed_end) return false;
    }
    const r = @wasmMemoryGrow(0, want_pages);
    if (r < 0) return false;
    return true;
}

pub fn alloc(size: u32) callconv(.c) u32 {
    const requested = (size + 7) & ~@as(u32, 7);
    // Fast path for the dominant case — OBJ_SIZE TclObj headers
    // hit the class-0 free-list directly without scanning the
    // SIZE_CLASSES table.  Every other size goes through the
    // generic class lookup.
    if (requested == OBJ_SIZE) {
        if (free_lists[0] != 0) {
            const ptr = free_lists[0];
            free_lists[0] = @intCast(read_i32(ptr));
            return ptr;
        }
        const ptr = heap_ptr;
        const end = ptr + OBJ_SIZE;
        if (!ensure_capacity(end)) {
            oom_flag = 1;
            return 0;
        }
        heap_ptr = end;
        return ptr;
    }
    const aligned = round_up_to_class(requested);
    // Try the matching size-class free-list first.
    if (class_head_for(aligned)) |head_ptr| {
        if (head_ptr.* != 0) {
            const ptr = head_ptr.*;
            head_ptr.* = @intCast(read_i32(ptr));
            return ptr;
        }
    }
    const ptr = heap_ptr;
    const end = ptr + aligned;
    if (!ensure_capacity(end)) {
        oom_flag = 1;
        return 0;
    }
    heap_ptr = end;
    return ptr;
}

pub export fn tcl_test_heap_ptr() i32 {
    return @bitCast(heap_ptr);
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
    // The TclObj header is OBJ_SIZE — recycle into class-0.
    write_i32(addr, @intCast(free_lists[0]));
    free_lists[0] = addr;
}

/// Public free-by-size: callers that own a non-OBJ_SIZE slab (string
/// buffer, list backing, dict pair table, frame table) push it onto
/// the matching size-class free-list at the end of its lifetime.
/// Slabs whose aligned size doesn't match any class are dropped on
/// the floor — the bump allocator's ``heap_ptr`` is the only resource
/// that grows, and the next allocation will pull from a fresh page.
/// That's wasteful but safe; sub-plan 3.1 + 3.2 will eliminate the
/// allocations that produce these classes by holding capacity in
/// place.
pub fn free_sized(addr: u32, size: u32) void {
    if (addr == 0) return;
    const aligned = (size + 7) & ~@as(u32, 7);
    if (class_head_for(aligned)) |head_ptr| {
        write_i32(addr, @intCast(head_ptr.*));
        head_ptr.* = addr;
    }
    // else: slab too big for any class — leak to bump pointer.
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

// Allocate a new TclObj with refcount 1
pub fn obj_alloc() u32 {
    const ptr = alloc(OBJ_SIZE);
    write_i32(ptr + OBJ_REFCOUNT, 1);
    write_i32(ptr + OBJ_TYPE_TAG, TYPE_STRING);
    write_i64(ptr + OBJ_INT_CACHE, 0);
    write_i32(ptr + OBJ_STR_PTR, 0);
    write_i32(ptr + OBJ_STR_LEN, 0);
    // Default cap = 0 means "we don't own the buffer".  Buffer-
    // owning paths (obj_new_string_copy, the in-place append grower)
    // overwrite this with the actual allocation size.
    write_i32(ptr + OBJ_STR_CAP, 0);
    return ptr;
}

pub export fn obj_new_int(value: i64) i32 {
    const ptr = obj_alloc();
    write_i32(ptr + OBJ_TYPE_TAG, TYPE_INT);
    write_i64(ptr + OBJ_INT_CACHE, value);
    return @as(i32, @intCast(ptr));
}

pub export fn obj_new_string(data_ptr: i32, length: i32) i32 {
    const ptr = obj_alloc();
    write_i32(ptr + OBJ_TYPE_TAG, TYPE_STRING);
    write_i32(ptr + OBJ_STR_PTR, data_ptr);
    write_i32(ptr + OBJ_STR_LEN, length);
    return @as(i32, @intCast(ptr));
}

pub export fn obj_new_float(value: f64) i32 {
    const ptr = obj_alloc();
    write_i32(ptr + OBJ_TYPE_TAG, TYPE_FLOAT);
    write_i64(ptr + OBJ_INT_CACHE, @bitCast(value));
    return @as(i32, @intCast(ptr));
}

pub export fn obj_get_float(obj: i32) f64 {
    if (obj == 0) return 0.0;
    const addr: u32 = @intCast(obj);
    const tag = read_i32(addr + OBJ_TYPE_TAG);
    if (tag == TYPE_FLOAT) return @bitCast(read_i64(addr + OBJ_INT_CACHE));
    if (tag == TYPE_INT) return @floatFromInt(read_i64(addr + OBJ_INT_CACHE));
    if (tag == TYPE_STRING) {
        const sptr: u32 = @intCast(read_i32(addr + OBJ_STR_PTR));
        const slen: u32 = @intCast(read_i32(addr + OBJ_STR_LEN));
        if (try_parse_float(sptr, slen)) |val| return val;
        if (try_parse_int(sptr, slen)) |val| return @floatFromInt(val);
    }
    return 0.0;
}

pub export fn obj_get_int(obj: i32) i64 {
    // Null/zero pointer sentinel — return 0 rather than reading
    // arbitrary bytes from address 0 (the pre-heap WASM stack area).
    // ``global_get`` returns 0 for unset variables; callers that pass
    // that result here expect to get the empty-string / zero integer.
    if (obj == 0) return 0;
    const addr: u32 = @intCast(obj);
    const tag = read_i32(addr + OBJ_TYPE_TAG);
    if (tag == TYPE_INT) return read_i64(addr + OBJ_INT_CACHE);
    if (tag == TYPE_FLOAT) {
        const fval: f64 = @bitCast(read_i64(addr + OBJ_INT_CACHE));
        return @intFromFloat(fval);
    }
    if (tag == TYPE_STRING) {
        const sptr: u32 = @intCast(read_i32(addr + OBJ_STR_PTR));
        const slen: u32 = @intCast(read_i32(addr + OBJ_STR_LEN));
        if (try_parse_int(sptr, slen)) |val| {
            write_i32(addr + OBJ_TYPE_TAG, TYPE_INT);
            write_i64(addr + OBJ_INT_CACHE, val);
            return val;
        }
        // Float string: parse and truncate to integer (Tcl semantics:
        // ``expr {int("2.7")}`` = 2).  Do not cache as TYPE_INT since
        // the value retains its fractional form.
        if (try_parse_float(sptr, slen)) |fval| {
            return @intFromFloat(fval);
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
    var val: i64 = 0;
    while (i < len and src[i] >= '0' and src[i] <= '9') {
        val = val * 10 + @as(i64, src[i] - '0');
        i += 1;
    }
    while (i < len and is_space(src[i])) i += 1;
    if (i != len) return null;
    return if (negative) -val else val;
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
        if (c >= '0' and c <= '9') { has_digit = true; i += 1; }
        else if (c == '.' and !has_dot and !has_exp) { has_dot = true; i += 1; }
        else if ((c == 'e' or c == 'E') and !has_exp and has_digit) {
            has_exp = true;
            i += 1;
            if (i < len and (src[i] == '+' or src[i] == '-')) i += 1;
        }
        else break;
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
    const addr: u32 = @intCast(obj);
    const rc = read_i32(addr + OBJ_REFCOUNT);
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
const PENDING_FREE_CAP: u32 = 256;
var pending_free: [PENDING_FREE_CAP]u32 = [_]u32{0} ** PENDING_FREE_CAP;
var pending_free_count: u32 = 0;

fn release_now(addr: u32) void {
    const cap: u32 = @bitCast(read_i32(addr + OBJ_STR_CAP));
    if (cap > 0) {
        const sp: u32 = @bitCast(read_i32(addr + OBJ_STR_PTR));
        if (sp != 0) free_sized(sp, cap);
    }
    free_obj(addr);
}

/// Drain the deferred-free queue.  Called by the eval loop between
/// statements so all references to a since-released TclObj's bytes
/// have been consumed before the slab is reissued.
pub export fn tcl_obj_drain_pending() void {
    var i: u32 = 0;
    while (i < pending_free_count) : (i += 1) {
        release_now(pending_free[i]);
    }
    pending_free_count = 0;
}

pub export fn tcl_obj_release(obj: i32) void {
    if (obj == 0) return;
    const addr: u32 = @intCast(obj);
    const rc = read_i32(addr + OBJ_REFCOUNT);
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
    const addr: u32 = @intCast(obj);
    return @intCast(read_i32(addr + OBJ_STR_PTR));
}

pub fn obj_str_len(obj: i32) u32 {
    const addr: u32 = @intCast(obj);
    return @intCast(read_i32(addr + OBJ_STR_LEN));
}

pub fn obj_type(obj: i32) i32 {
    const addr: u32 = @intCast(obj);
    return read_i32(addr + OBJ_TYPE_TAG);
}

/// Copy *len* bytes from src to dst in linear memory.
pub fn memcpy(dst: u32, src: u32, len: u32) void {
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
    // Round capacity up to the smallest size class so the recycler
    // can return the slab to the right free-list at end-of-life.
    const cap = round_up_to_class((len + 7) & ~@as(u32, 7));
    const buf = alloc(cap);
    if (buf == 0) return 0;
    memcpy(buf, src, len);
    const obj = obj_new_string(@intCast(buf), @intCast(len));
    if (obj != 0) {
        write_i32(@as(u32, @intCast(obj)) + OBJ_STR_CAP, @intCast(cap));
    }
    return obj;
}

// Scratch buffer for integer-to-string conversion (no newline)
var itoa_buf: [21]u8 = undefined;

pub fn itoa(value: i64) struct { ptr: [*]u8, len: u32 } {
    var v = value;
    var negative = false;
    if (v < 0) {
        negative = true;
        v = -v;
    }
    var i: u32 = itoa_buf.len - 1;
    if (v == 0) {
        itoa_buf[i] = '0';
    } else {
        while (v > 0) {
            itoa_buf[i] = @as(u8, @intCast(@rem(v, 10))) + '0';
            v = @divTrunc(v, 10);
            if (v > 0) i -= 1;
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
    const result = std.fmt.bufPrint(&ftoa_buf, "{d}", .{value}) catch ftoa_buf[0..1];
    const len = result.len;
    // Tcl requires floats to look like floats: ensure the string contains
    // a '.', 'e', or 'E' so that "5.0" is not confused with integer "5".
    var has_dot = false;
    for (result) |c| {
        if (c == '.' or c == 'e' or c == 'E') { has_dot = true; break; }
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
    const addr: u32 = @intCast(obj);
    const tag = read_i32(addr + OBJ_TYPE_TAG);
    if (tag == TYPE_STRING) {
        return .{
            .ptr = @intCast(read_i32(addr + OBJ_STR_PTR)),
            .len = @intCast(read_i32(addr + OBJ_STR_LEN)),
        };
    }
    const sptr: u32 = @intCast(read_i32(addr + OBJ_STR_PTR));
    if (sptr != 0) {
        return .{
            .ptr = sptr,
            .len = @intCast(read_i32(addr + OBJ_STR_LEN)),
        };
    }
    if (tag == TYPE_FLOAT) {
        const fval: f64 = @bitCast(read_i64(addr + OBJ_INT_CACHE));
        const result = ftoa(fval);
        const buf = alloc(result.len);
        memcpy(buf, @intFromPtr(result.ptr), result.len);
        write_i32(addr + OBJ_STR_PTR, @intCast(buf));
        write_i32(addr + OBJ_STR_LEN, @intCast(result.len));
        return .{ .ptr = buf, .len = result.len };
    }
    const val = read_i64(addr + OBJ_INT_CACHE);
    const result = itoa(val);
    const buf = alloc(result.len);
    memcpy(buf, @intFromPtr(result.ptr), result.len);
    write_i32(addr + OBJ_STR_PTR, @intCast(buf));
    write_i32(addr + OBJ_STR_LEN, @intCast(result.len));
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
