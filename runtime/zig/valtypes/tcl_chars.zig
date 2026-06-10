// Character classification and byte-span comparison helpers shared by
// every parsing / scanning module in the runtime.
//
// Mirrors `generic/tclParse.c` from reference Tcl 9.0: the classifiers
// (`is_space`, `is_scan_space`, `is_bareword`, `is_digit` ...) replace
// repeated inline byte checks in callers, and the ``str_*`` / ``mem_*``
// helpers collapse the two copies of the comptime equality comparator
// that had accreted (one in `tcl_interp.zig`, one in `tcl_cmd_info.zig`).
//
// All functions are pure predicates / comparators — no allocation, no
// TclObj layer.  Any runtime module should ``@import("tcl_chars.zig")``
// rather than hand-rolling a new classifier: the whole point of this
// module is to keep the set of "what counts as whitespace in Tcl"
// answers in one place, so a fix lands everywhere or nowhere.

/// ASCII whitespace set used by Tcl list parsing / field-splitting.
/// Matches the byte set on which ``TclFindElement`` splits — space,
/// tab, LF, CR.  (Tcl's ``TclIsSpaceProcM`` also includes vertical
/// tab / form feed, but the runtime's list parser has never
/// classified those as separators; widening without auditing every
/// caller would change tokenisation mid-pipeline.)
pub fn is_space(c: u8) bool {
    return c == ' ' or c == '\t' or c == '\n' or c == '\r';
}

/// Extended whitespace set used by ``TclScanElement`` — the braceable-
/// whitespace detector in `tcl_list_quote.scan_element` treats
/// ``\v`` / ``\f`` as word-terminators, matching reference Tcl.
pub fn is_scan_space(c: u8) bool {
    return c == ' ' or c == '\t' or c == '\n' or c == 0x0B or c == 0x0C or c == '\r';
}

/// A decimal digit ``0``-``9``.
pub fn is_digit(c: u8) bool {
    return c >= '0' and c <= '9';
}

/// A hexadecimal digit ``0``-``9`` / ``a``-``f`` / ``A``-``F``.
pub fn is_hex_digit(c: u8) bool {
    return (c >= '0' and c <= '9') or
        (c >= 'a' and c <= 'f') or
        (c >= 'A' and c <= 'F');
}

/// An octal digit ``0``-``7``.
pub fn is_octal_digit(c: u8) bool {
    return c >= '0' and c <= '7';
}

/// A bareword character — alphanumeric plus ``_`` and ``:``.  Matches
/// the set ``Tcl_ParseVarName`` accepts for unbraced ``$name``
/// identifiers (``:`` is for namespace-qualified names like ``ns::var``).
pub fn is_bareword(c: u8) bool {
    return (c >= 'a' and c <= 'z') or
        (c >= 'A' and c <= 'Z') or
        (c >= '0' and c <= '9') or
        c == '_' or c == ':';
}

/// Compile-time-constant byte-slice equality — the common case is
/// comparing a runtime byte span against a static Tcl keyword like
/// ``"lappend"`` / ``"uplevel"``.  Callers pass the keyword as a
/// ``comptime []const u8`` so the length check collapses to a constant
/// and the byte-by-byte loop unrolls.
pub fn str_eq(a: [*]const u8, alen: u32, comptime b: []const u8) bool {
    if (alen != b.len) return false;
    inline for (0..b.len) |i| {
        if (a[i] != b[i]) return false;
    }
    return true;
}

/// Runtime byte-slice equality — used when both operands are dynamic.
pub fn mem_eq(a: [*]const u8, alen: u32, b: [*]const u8, blen: u32) bool {
    if (alen != blen) return false;
    for (0..alen) |i| {
        if (a[i] != b[i]) return false;
    }
    return true;
}

/// Runtime Zig-slice equality — convenience wrapper used by callers that
/// already have slices (eg parsed-flag arrays in ``try_parse_bool``).
pub fn slice_eq(a: []const u8, b: []const u8) bool {
    if (a.len != b.len) return false;
    for (a, b) |ca, cb| {
        if (ca != cb) return false;
    }
    return true;
}

/// Compare two byte spans lexicographically.  Returns <0, 0, or >0
/// matching `memcmp` / `Tcl_CmpObj` conventions.  Used by list sort
/// paths and string comparison commands.
pub fn str_cmp(a_ptr: u32, a_len: u32, b_ptr: u32, b_len: u32) i32 {
    // ``@ptrFromInt(0)`` panics on a non-optional pointer in
    // ReleaseSafe; treat null/zero pointers with len=0 as the
    // empty string.  Callers like ``tcl_cmd_list_contains`` reach
    // here with a null TclObj value (``obj_ensure_string(0)``
    // returns ptr=0, len=0).
    const min_len = if (a_len < b_len) a_len else b_len;
    if (min_len > 0) {
        const pa: [*]const u8 = @ptrFromInt(a_ptr);
        const pb: [*]const u8 = @ptrFromInt(b_ptr);
        for (0..min_len) |k| {
            if (pa[k] < pb[k]) return -1;
            if (pa[k] > pb[k]) return 1;
        }
    }
    if (a_len < b_len) return -1;
    if (a_len > b_len) return 1;
    return 0;
}
