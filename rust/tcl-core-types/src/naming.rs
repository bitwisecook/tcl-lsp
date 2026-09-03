//! Dependency-free Tcl name scanners shared by lexer and runtime crates.

/// Scan the unbraced variable-name extent beginning at `start` (the first byte
/// after `$`).  Tcl's `Tcl_ParseVarName` consumes an entire run of two or more
/// colons as one namespace separator, including any further colons in that
/// run: `$a:::b` therefore scans through `a:::b`, and `$foo:::` through the
/// trailing separator naming the empty tail.  ASCII name characters are used
/// deliberately; UTF-8 bytes terminate the scan without splitting a codepoint.
#[must_use]
pub fn scan_var_name_end(src: &[u8], start: usize) -> usize {
    scan_var_name_end_with(src, start, false)
}

/// [`scan_var_name_end`], with `JimTcl`'s wider bareword class available.
///
/// When `allow_high_bytes` is true, every byte >= 0x80 also continues the
/// name. `JimParseVar` (`jim.c:1691`) accepts `isalnum`, `_`, and
/// `UCHAR(*pc->p) >= 0x80` — its comment reads "we consider all unicode
/// points outside of ASCII as letters" — so `$café` is one variable there
/// and names the variable `caf` in every build of the Tcl core, where
/// `TclIsBareword` is ASCII-only.
///
/// Testing raw bytes rather than decoded scalars is deliberate and matches
/// Jim: every byte of a UTF-8 multi-byte sequence is >= 0x80, so a whole
/// sequence is consumed without decoding it, and a name can never end
/// mid-character.
#[must_use]
pub fn scan_var_name_end_with(src: &[u8], start: usize, allow_high_bytes: bool) -> usize {
    let mut p = start;
    while p < src.len() {
        let c = src[p];
        if c.is_ascii_alphanumeric() || c == b'_' || (allow_high_bytes && c >= 0x80) {
            p += 1;
        } else if c == b':' && p + 1 < src.len() && src[p + 1] == b':' {
            p += 2;
            while p < src.len() && src[p] == b':' {
                p += 1;
            }
        } else {
            break;
        }
    }
    p
}

#[cfg(test)]
mod tests {
    use super::scan_var_name_end;

    #[test]
    fn consumes_complete_separator_runs() {
        for (src, end) in [(b"a:::b".as_slice(), 5), (b"::a:::b", 7), (b"foo:::", 6)] {
            assert_eq!(scan_var_name_end(src, 0), end);
        }
        assert_eq!(scan_var_name_end(b"a:::b(x)", 0), 5);
        assert_eq!(scan_var_name_end("a::é".as_bytes(), 0), 3);
    }
}
