// Tcl list parser — walks a Tcl list string and extracts elements.
//
// The input side of the list-string contract.  Ports the reference
// Tcl 9.0 ``TclFindElement`` / ``Tcl_SplitList`` loop (from
// ``generic/tclUtil.c``) using two small primitives instead of one
// monolithic ``TclFindElement``:
//
//   - :func:`count_elements` — how many elements are in this list?
//   - :func:`element_at`     — start/len/braced flag for the nth element.
//
// Splitting the job this way matches how our runtime actually uses
// list parsing — most call sites want ``list_count_elements`` once
// followed by repeated ``list_element_at(i)`` calls for random-access
// iteration.
//
// An element returned with ``braced == true`` must NOT have backslash
// substitution applied — it's already literal inside the outer
// ``{...}``.  Unbraced elements ARE subject to the Tcl backslash rules
// and should pass through :func:`copy_unbraced_elem`, which delegates
// to ``tcl_bs.decode_into``.
//
// Layering: shares ``tcl_chars.is_space`` with every other parsing
// module.  Shares ``tcl_bs`` for backslash decoding.  No TclObj /
// allocator dependency — the caller owns the output buffer.

const chars = @import("tcl_chars.zig");
const bs = @import("tcl_bs.zig");

const is_space = chars.is_space;

pub const Element = struct {
    start: u32,
    len: u32,
    braced: bool,
};

/// Count whitespace-separated elements in *src*.  Braced ``{...}``
/// elements count as one; quoted ``"..."`` elements when ``"`` starts
/// an element similarly count as one (matching Tcl's list-syntax
/// rules).  Backslash-escaped ``\{`` / ``\}`` / ``\"`` inside an
/// unbraced/unquoted word do NOT split.
pub fn count_elements(ptr: u32, len: u32) i64 {
    if (len == 0 or ptr == 0) return 0;
    const src: [*]const u8 = @ptrFromInt(ptr);
    var count: i64 = 0;
    var i: u32 = 0;
    while (i < len) {
        while (i < len and is_space(src[i])) i += 1;
        if (i >= len) break;
        count += 1;
        if (src[i] == '{') {
            i += 1;
            var depth: u32 = 1;
            while (i < len and depth > 0) {
                // Tcl list parsing: a backslash escapes the next
                // character so ``\{`` / ``\}`` do NOT affect the
                // brace-nesting depth.  Consume both bytes and
                // continue without touching depth.
                if (src[i] == '\\' and i + 1 < len) {
                    i += 2;
                    continue;
                }
                if (src[i] == '{') {
                    depth += 1;
                } else if (src[i] == '}') {
                    depth -= 1;
                }
                i += 1;
            }
        } else if (src[i] == '"') {
            // Quoted list element — closes on the next unescaped
            // ``"``.  Tcl treats a leading ``"`` as a quoting
            // character only when it starts an element (the
            // surrounding whitespace already ensures that).  See
            // ``TclFindElement`` in upstream ``generic/tclUtil.c``.
            i += 1;
            while (i < len) {
                if (src[i] == '\\' and i + 1 < len) {
                    i += 2;
                    continue;
                }
                if (src[i] == '"') {
                    i += 1;
                    break;
                }
                i += 1;
            }
        } else {
            while (i < len and !is_space(src[i])) {
                if (src[i] == '\\' and i + 1 < len) {
                    i += 2;
                } else {
                    i += 1;
                }
            }
        }
    }
    return count;
}

/// Validate that a list-string has balanced braces.  Returns true
/// when the input is well-formed.  Internal callers
/// (``count_elements`` / ``element_at``) stay permissive — they use
/// the count as a sentinel and a partial walk is fine.  Only the
/// user-visible ``tcl_cmd_list_length`` (S2.7) wants this check
/// because ``llength`` reports the unmatched-brace error to users
/// while ``linsert`` / ``lrepeat`` / ``lseq`` continue to operate
/// on whatever string they're given.
pub fn validate_list_braces(ptr: u32, len: u32) bool {
    if (len == 0 or ptr == 0) return true;
    const src: [*]const u8 = @ptrFromInt(ptr);
    var i: u32 = 0;
    while (i < len) {
        while (i < len and is_space(src[i])) i += 1;
        if (i >= len) break;
        if (src[i] == '{') {
            i += 1;
            var depth: u32 = 1;
            while (i < len and depth > 0) {
                if (src[i] == '\\' and i + 1 < len) {
                    i += 2;
                    continue;
                }
                if (src[i] == '{') {
                    depth += 1;
                } else if (src[i] == '}') {
                    depth -= 1;
                }
                i += 1;
            }
            if (depth > 0) return false;
        } else {
            while (i < len and !is_space(src[i])) {
                if (src[i] == '\\' and i + 1 < len) {
                    i += 2;
                } else {
                    i += 1;
                }
            }
        }
    }
    return true;
}

/// Validate that a string is a well-formed Tcl list.  Returns 0 on success.
/// On failure, raises an error via ``stubs.raise`` and returns -1.
/// Specifically detects the ``{elem}{next}`` case (brace element not
/// followed by whitespace or end) which ``count_elements`` silently
/// ignores but Tcl 9 reports as ``list element in braces followed by
/// "<token>" instead of space``.
pub fn check_list_syntax(ptr: u32, len: u32) i32 {
    if (len == 0 or ptr == 0) return 0;
    const src: [*]const u8 = @ptrFromInt(ptr);
    const stubs = @import("../stubs/tcl_stubs.zig");
    const obj = @import("tcl_obj.zig");
    var i: u32 = 0;
    while (i < len) {
        while (i < len and is_space(src[i])) i += 1;
        if (i >= len) break;
        if (src[i] == '{') {
            i += 1;
            var depth: u32 = 1;
            while (i < len and depth > 0) {
                if (src[i] == '\\' and i + 1 < len) {
                    i += 2;
                    continue;
                }
                if (src[i] == '{') depth += 1
                else if (src[i] == '}') depth -= 1;
                i += 1;
            }
            if (depth > 0) {
                stubs.raise("unmatched open brace in list");
                return -1;
            }
            // After closing '}', next char must be whitespace or end.
            if (i < len and !is_space(src[i])) {
                // Build error: ``list element in braces followed by "<tok>" instead of space``
                // Capture the offending token (up to first whitespace or end).
                const tok_start = i;
                var tok_end = i;
                while (tok_end < len and !is_space(src[tok_end])) tok_end += 1;
                const tok_len = tok_end - tok_start;
                // Build the error message as a TclObj string.
                const prefix = "list element in braces followed by \"";
                const suffix = "\" instead of space";
                const total: u32 = @intCast(prefix.len + tok_len + suffix.len);
                const buf_addr: u32 = obj.alloc(total);
                if (buf_addr == 0) {
                    stubs.raise("list element in braces followed by something instead of space");
                    return -1;
                }
                const buf: [*]u8 = @ptrFromInt(buf_addr);
                var off: u32 = 0;
                for (prefix) |c| { buf[off] = c; off += 1; }
                for (0..tok_len) |k| { buf[off] = src[tok_start + k]; off += 1; }
                for (suffix) |c| { buf[off] = c; off += 1; }
                // Route through catch_mod directly (same as stubs.raise but
                // with a pre-built buffer we already own).
                const catch_mod = @import("../interp/tcl_catch.zig");
                const msg_obj2 = obj.obj_new_string_take(buf_addr, total, total);
                catch_mod.tcl_cmd_error(msg_obj2);
                return -1;
            }
        } else if (src[i] == '"') {
            i += 1;
            while (i < len) {
                if (src[i] == '\\' and i + 1 < len) { i += 2; continue; }
                if (src[i] == '"') { i += 1; break; }
                i += 1;
            }
        } else {
            while (i < len and !is_space(src[i])) {
                if (src[i] == '\\' and i + 1 < len) { i += 2; }
                else i += 1;
            }
        }
    }
    return 0;
}

/// Return the (start, len, braced) span of the nth element (0-based).
/// Out-of-range requests return a zero-length element so callers can
/// treat them as empty without a separate ``is_valid`` check.
pub fn element_at(ptr: u32, len: u32, idx: i64) Element {
    if (len == 0 or ptr == 0) return .{ .start = 0, .len = 0, .braced = false };
    const src: [*]const u8 = @ptrFromInt(ptr);
    var count: i64 = 0;
    var i: u32 = 0;
    while (i < len) {
        while (i < len and is_space(src[i])) i += 1;
        if (i >= len) break;
        if (src[i] == '{') {
            i += 1;
            const inner_start = i;
            var depth: u32 = 1;
            while (i < len and depth > 0) {
                // Backslash escapes the next char — ``\{`` / ``\}``
                // inside a braced list element are NOT depth-changing.
                if (src[i] == '\\' and i + 1 < len) {
                    i += 2;
                    continue;
                }
                if (src[i] == '{') {
                    depth += 1;
                } else if (src[i] == '}') {
                    depth -= 1;
                }
                i += 1;
            }
            if (count == idx) {
                // Balanced close — subtract the trailing ``}`` from
                // the content span.  Unterminated input (``depth > 0``)
                // leaves no trailing ``}``, so ``i`` points past the
                // last consumed byte; skipping this guard would
                // u32-underflow when ``i == inner_start``.
                const elem_len = if (depth == 0) i - 1 - inner_start else i - inner_start;
                return .{ .start = inner_start, .len = elem_len, .braced = true };
            }
        } else if (src[i] == '"') {
            // Quoted element ``"..."`` — span runs from the byte
            // after the opening ``"`` to the byte before the closing
            // ``"``.  Backslash-escaped ``\"`` doesn't terminate.
            // Marked ``braced = false`` because the content is still
            // subject to backslash substitution (Tcl semantics:
            // quoted elements receive backslash decoding, just like
            // unquoted ones).
            i += 1;
            const inner_start = i;
            var closed = false;
            while (i < len) {
                if (src[i] == '\\' and i + 1 < len) {
                    i += 2;
                    continue;
                }
                if (src[i] == '"') {
                    closed = true;
                    break;
                }
                i += 1;
            }
            if (count == idx) {
                const elem_len = i - inner_start;
                if (closed) i += 1;
                return .{ .start = inner_start, .len = elem_len, .braced = false };
            }
            if (closed) i += 1;
        } else {
            const elem_start = i;
            while (i < len and !is_space(src[i])) {
                if (src[i] == '\\' and i + 1 < len) {
                    i += 2;
                } else {
                    i += 1;
                }
            }
            if (count == idx) {
                return .{ .start = elem_start, .len = i - elem_start, .braced = false };
            }
        }
        count += 1;
    }
    return .{ .start = 0, .len = 0, .braced = false };
}

/// Sequential list cursor — advances one element at a time in O(1) per
/// step, avoiding the O(N) rescan that :func:`element_at` would incur
/// when called in a loop.  Callers that iterate all elements should
/// prefer this over repeated ``element_at(i)`` calls.
pub const Cursor = struct {
    pos: u32,
};

/// Advance *cursor* past the next whitespace-separated element.
/// Returns the element span (offsets relative to ``ptr``).
/// When the list is exhausted returns a zero-length element with
/// ``cursor.pos`` set to ``len``.
pub fn cursor_next(ptr: u32, len: u32, cursor: *Cursor) Element {
    if (ptr == 0 or cursor.pos >= len)
        return .{ .start = 0, .len = 0, .braced = false };
    const src: [*]const u8 = @ptrFromInt(ptr);
    var i: u32 = cursor.pos;
    while (i < len and is_space(src[i])) i += 1;
    if (i >= len) {
        cursor.pos = len;
        return .{ .start = 0, .len = 0, .braced = false };
    }
    if (src[i] == '{') {
        i += 1;
        const inner_start = i;
        var depth: u32 = 1;
        while (i < len and depth > 0) {
            if (src[i] == '\\' and i + 1 < len) { i += 2; continue; }
            if (src[i] == '{') depth += 1
            else if (src[i] == '}') depth -= 1;
            i += 1;
        }
        cursor.pos = i;
        const elem_len = if (depth == 0) i - 1 - inner_start else i - inner_start;
        return .{ .start = inner_start, .len = elem_len, .braced = true };
    } else if (src[i] == '"') {
        i += 1;
        const inner_start = i;
        var closed = false;
        while (i < len) {
            if (src[i] == '\\' and i + 1 < len) { i += 2; continue; }
            if (src[i] == '"') { closed = true; break; }
            i += 1;
        }
        const elem_len = i - inner_start;
        if (closed) i += 1;
        cursor.pos = i;
        return .{ .start = inner_start, .len = elem_len, .braced = false };
    } else {
        const elem_start = i;
        while (i < len and !is_space(src[i])) {
            if (src[i] == '\\' and i + 1 < len) i += 2
            else i += 1;
        }
        cursor.pos = i;
        return .{ .start = elem_start, .len = i - elem_start, .braced = false };
    }
}

/// Skip *count* elements starting from *cursor.pos*.  Cheaper than
/// calling :func:`cursor_next` and discarding results when jumping
/// forward by ``stride`` steps.
pub fn cursor_skip(ptr: u32, len: u32, cursor: *Cursor, count: u32) void {
    var k: u32 = 0;
    while (k < count) : (k += 1) {
        const e = cursor_next(ptr, len, cursor);
        if (e.len == 0 and cursor.pos >= len) break;
    }
}

/// Copy an unbraced list-element's bytes, expanding backslash sequences
/// via :func:`tcl_bs.decode_into`.  Returns the number of output bytes
/// written to ``dst``.  Braced elements must NOT be passed through this
/// — they are already literal and should be copied verbatim by the
/// caller.
pub fn copy_unbraced_elem(dst: u32, src_ptr: u32, src_len: u32) u32 {
    return bs.decode_into(src_ptr, src_len, dst);
}
