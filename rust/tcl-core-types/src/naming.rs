// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Dependency-free Tcl name scanners shared by lexer and runtime crates.

/// Whether the supplied counted bytes contain a written namespace separator.
/// A separator is a run of two or more colons. Callers independently select
/// the input extent and naming purpose; this predicate supplies no C/Jim
/// lookup policy, namespace identity, canonicalisation or Native authority.
#[must_use]
pub fn has_namespace_separator(name: &[u8]) -> bool {
    name.windows(2).any(|pair| pair == b"::")
}

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
