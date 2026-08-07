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

//! Tcl list grammar — `Tcl_SplitList` (split) and `Tcl_Merge` (join), the
//! canonical pair shared by the compiler's const-folder, the LSP, and the
//! runtime's list value type + `{*}` expansion.
//!
//! Re-derived from reference Tcl 9.0 `tmp/tcl9.0.3/generic/tclUtil.c`:
//! `Tcl_SplitList`/`TclFindElement`/`FindElement` for the split (including the
//! `literal` zero-copy flag and `TclCopyAndCollapse`), and
//! `Tcl_ScanElement`/`Tcl_ConvertElement` (`ConvertFlags`) for the join.
//!
//! Backslash decoding is **not** re-implemented here: bare/quoted elements that
//! need collapsing run through [`tcl_lexer::backslash_subst`] (the one canonical
//! decoder, matching `TclParseBackslash`).
//!
//! ## Split utilities
//!
//! [`find_element`] is the single grammar primitive; every splitter is a thin
//! layer over it, so callers should reach for one of these rather than hand-roll
//! another brace/quote/backslash scan:
//!
//! - [`split_list`] — decoded element *values* (`Tcl_SplitList`), strict.
//! - [`split_list_raw`] — verbatim element *text* (no backslash decode), strict —
//!   for re-emitting the original literal.
//! - [`split_list_lenient`] / [`split_list_raw_lenient`] — the same two, but
//!   returning the elements parsed before a malformed tail instead of `Err`.
//!
//! ## The list grammar (`FindElement`)
//!
//! - Leading/trailing element whitespace is space/tab/newline/CR/FF/VT (note:
//!   `;` is **not** a list separator — that is command syntax).
//! - `{braced}` elements group with **nested** braces and are taken **verbatim**
//!   (no substitution, no backslash collapse — `\}` etc. stay literal).
//! - `"quoted"` and bare elements have backslash escapes **collapsed**; a
//!   backslash escapes the next byte (so `\"`/`\{`/`\ ` do not terminate).
//! - A close brace/quote must be followed by element whitespace or end-of-input,
//!   else it is "list element in braces/quotes followed by ...".
//! - An unterminated `{`/`"` is "unmatched open brace/quote in list".

use std::borrow::Cow;
use std::ops::Range;

use tcl_lexer::backslash_subst;

/// Why splitting a string as a Tcl list failed (the `tclUtil.c` error set).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListError {
    /// `unmatched open brace in list` — a `{` with no matching `}`.
    UnmatchedBrace,
    /// `unmatched open quote in list` — a `"` with no matching `"`.
    UnmatchedQuote,
    /// `list element in braces followed by "..." instead of space` — junk
    /// directly after a closing `}`.
    BraceFollowedByJunk,
    /// `list element in quotes followed by "..." instead of space` — junk
    /// directly after a closing `"`.
    QuoteFollowedByJunk,
}

impl ListError {
    /// The reference Tcl error message (prefix; callers append the offending
    /// fragment for the "...followed by" cases if they want byte-exact text).
    #[must_use]
    pub fn message(self) -> &'static str {
        match self {
            ListError::UnmatchedBrace => "unmatched open brace in list",
            ListError::UnmatchedQuote => "unmatched open quote in list",
            ListError::BraceFollowedByJunk => "list element in braces followed by",
            ListError::QuoteFollowedByJunk => "list element in quotes followed by",
        }
    }

    /// The complete Tcl error message for splitting `src` as a list. For the
    /// `…followed by "X" instead of space` cases this surfaces the offending
    /// fragment `X` directly after the closing delimiter (`tclUtil.c`); the other
    /// cases are fixed text.
    #[must_use]
    pub fn full_message(self, src: &str) -> String {
        let kind = match self {
            ListError::BraceFollowedByJunk => "braces",
            ListError::QuoteFollowedByJunk => "quotes",
            _ => return self.message().to_string(),
        };
        let frag = list_junk_fragment(src);
        format!("list element in {kind} followed by \"{frag}\" instead of space")
    }
}

/// The junk text immediately after the closing delimiter of the element that
/// failed to parse — the `"X"` in `…followed by "X" instead of space`. Walks the
/// successfully-parsed prefix to the failing element, past its closing `}`/`"`,
/// and returns the run of non-whitespace bytes there (capped, matching C).
fn list_junk_fragment(src: &str) -> String {
    let bytes = src.as_bytes();
    let len = bytes.len();
    // The failing element begins where the last successful element left off.
    let mut start = 0;
    while let Ok(Some(el)) = find_element(src, start) {
        start = el.next;
    }
    let mut pos = start;
    while pos < len && is_list_space(bytes[pos]) {
        pos += 1;
    }
    // Walk to the delimiter that closes this element, honouring `\`-escapes and
    // brace nesting; the junk starts at the next byte.
    let junk = match bytes.get(pos) {
        Some(b'{') => {
            let mut depth = 1usize;
            pos += 1;
            while pos < len && depth > 0 {
                match bytes[pos] {
                    b'\\' if pos + 1 < len => pos += 1,
                    b'{' => depth += 1,
                    b'}' => depth -= 1,
                    _ => {}
                }
                pos += 1;
            }
            pos
        }
        Some(b'"') => {
            pos += 1;
            while pos < len && bytes[pos] != b'"' {
                pos += if bytes[pos] == b'\\' { 2 } else { 1 };
            }
            pos + 1 // past the closing quote
        }
        _ => return String::new(),
    };
    let end = (junk + 20).min(len);
    let mut p = junk;
    while p < end && !is_list_space(bytes[p]) {
        p += 1;
    }
    src.get(junk..p).unwrap_or("").to_string()
}

/// One located list element: the interior byte range in the source, whether it
/// is `literal` (no backslash collapse needed — Tcl's `literal` flag), whether
/// it was written `{brace}`-delimited, and the offset to resume scanning at
/// (past trailing whitespace).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Element {
    /// Interior byte range (delimiters stripped).
    pub value: Range<usize>,
    /// `true` ⇒ take `s[value]` verbatim; `false` ⇒ collapse its backslashes.
    pub literal: bool,
    /// `true` when the element was written as `{…}`.  A braced element's
    /// value is its source text byte-for-byte at exactly `value`'s offsets —
    /// no escape ever collapses inside braces — which is what a consumer
    /// that re-parses the element *in place* (as script, as a nested list)
    /// and reports source spans back needs.  A bare or double-quoted element
    /// may be `literal` too (when it happens to contain no backslash), but
    /// that is a property of the one value, not of the shape.
    pub braced: bool,
    /// Resume offset for the next [`find_element`] call.
    pub next: usize,
}

/// Tcl list-element whitespace (`TclIsSpaceProcM`): space, tab, NL, CR, FF, VT.
#[inline]
#[must_use]
pub fn is_list_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
}

/// Locate the next list element in `s` at/after `start`. Returns `Ok(None)` when
/// only trailing whitespace remains. (`tclUtil.c:577`).
pub fn find_element(s: &str, start: usize) -> Result<Option<Element>, ListError> {
    let bytes = s.as_bytes();
    let len = bytes.len();
    let mut pos = start;

    // Skip leading element whitespace.
    while pos < len && is_list_space(bytes[pos]) {
        pos += 1;
    }
    if pos >= len {
        return Ok(None);
    }

    let mut open_braces: usize = 0;
    let mut in_quotes = false;
    let mut literal = true;
    let braced = bytes[pos] == b'{';
    let elem_start;
    match bytes[pos] {
        b'{' => {
            open_braces = 1;
            pos += 1;
            elem_start = pos;
        }
        b'"' => {
            in_quotes = true;
            pos += 1;
            elem_start = pos;
        }
        _ => elem_start = pos,
    }

    let size;
    loop {
        if pos >= len {
            if open_braces != 0 {
                return Err(ListError::UnmatchedBrace);
            }
            if in_quotes {
                return Err(ListError::UnmatchedQuote);
            }
            size = pos - elem_start;
            break;
        }
        match bytes[pos] {
            // Inside braces a `{` nests; outside, it is a literal char of a bare
            // element (the `_` arm).
            b'{' if open_braces != 0 => {
                open_braces += 1;
            }
            b'}' => {
                if open_braces == 1 {
                    size = pos - elem_start;
                    pos += 1;
                    if pos < len && !is_list_space(bytes[pos]) {
                        return Err(ListError::BraceFollowedByJunk);
                    }
                    break;
                } else if open_braces > 1 {
                    open_braces -= 1;
                }
                // open_braces == 0: literal `}` in a bare element.
            }
            // A closing quote ends a quoted element; outside quotes a `"` is a
            // literal char mid bare element (the `_` arm).
            b'"' if in_quotes => {
                size = pos - elem_start;
                pos += 1;
                if pos < len && !is_list_space(bytes[pos]) {
                    return Err(ListError::QuoteFollowedByJunk);
                }
                break;
            }
            b'\\' => {
                if open_braces == 0 {
                    // A backslash outside braces ⇒ the element needs collapsing.
                    literal = false;
                }
                if pos + 1 < len {
                    // `\<newline>` outside braces is the line-continuation
                    // escape: C `TclParseBackslash` collapses it AND the
                    // following run of spaces/tabs into a single space, so that
                    // whitespace belongs to the element and must not terminate
                    // it — `split_list("a\\\n b")` is one element `a b`, not two.
                    if open_braces == 0 && bytes[pos + 1] == b'\n' {
                        pos += 2;
                        while pos < len && matches!(bytes[pos], b' ' | b'\t') {
                            pos += 1;
                        }
                        continue;
                    }
                    // Skip the escaped byte so `\}` / `\"` / `\<space>` do not
                    // terminate. Only the immediately-following byte matters for
                    // boundary detection (longer escapes' trailing digits are
                    // plain text either way).
                    pos += 2;
                    continue;
                }
                // A trailing lone `\` falls through to EOF.
                pos += 1;
                continue;
            }
            ch if open_braces == 0 && !in_quotes && is_list_space(ch) => {
                size = pos - elem_start;
                break; // pos stays at the separator; trailing-skip handles it.
            }
            _ => {}
        }
        pos += 1;
    }

    // Skip trailing whitespace to the next element.
    while pos < len && is_list_space(bytes[pos]) {
        pos += 1;
    }
    Ok(Some(Element {
        value: elem_start..elem_start + size,
        literal,
        braced,
        next: pos,
    }))
}

/// Split `s` into its Tcl list elements (`Tcl_SplitList`), collapsing backslash
/// escapes in bare/quoted elements. Literal elements borrow `s`; collapsed ones
/// own a fresh `String`.
pub fn split_list(s: &str) -> Result<Vec<Cow<'_, str>>, ListError> {
    let mut out = Vec::new();
    let mut pos = 0;
    while let Some(el) = find_element(s, pos)? {
        let raw = &s[el.value.clone()];
        out.push(if el.literal {
            Cow::Borrowed(raw)
        } else {
            // backslash_subst returns Borrowed when there is nothing to do, but
            // a non-literal element always contains a backslash, so this owns.
            Cow::Owned(backslash_subst(raw).into_owned())
        });
        pos = el.next;
    }
    Ok(out)
}

/// Split `s` into its Tcl list elements **verbatim** — each element's raw
/// interior text (delimiters stripped, backslashes *not* decoded), borrowing
/// `s`. This is the split for consumers that re-emit the original literal or
/// keep raw text (a split-then-[`list_element`] round-trip reproduces the
/// source), where [`split_list`]'s backslash decoding would be wrong.
pub fn split_list_raw(s: &str) -> Result<Vec<&str>, ListError> {
    let mut out = Vec::new();
    let mut pos = 0;
    while let Some(el) = find_element(s, pos)? {
        out.push(&s[el.value.clone()]);
        pos = el.next;
    }
    Ok(out)
}

/// [`split_list`] that never fails: on a malformed tail (unmatched brace/quote,
/// junk after a delimiter) it returns the elements parsed *before* the error
/// rather than `Err`. For best-effort consumers (constant folding, `llength` /
/// `in` estimates) that would rather see a partial list than nothing.
#[must_use]
pub fn split_list_lenient(s: &str) -> Vec<Cow<'_, str>> {
    let mut out = Vec::new();
    let mut pos = 0;
    while let Ok(Some(el)) = find_element(s, pos) {
        let raw = &s[el.value.clone()];
        out.push(if el.literal {
            Cow::Borrowed(raw)
        } else {
            Cow::Owned(backslash_subst(raw).into_owned())
        });
        pos = el.next;
    }
    out
}

/// [`split_list_raw`] that never fails, mirroring [`split_list_lenient`]'s
/// error tolerance — verbatim elements up to the first malformed tail.
#[must_use]
pub fn split_list_raw_lenient(s: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut pos = 0;
    while let Ok(Some(el)) = find_element(s, pos) {
        out.push(&s[el.value.clone()]);
        pos = el.next;
    }
    out
}

/// The largest number of elements `s` could hold as a list: a count of
/// whitespace-separated runs, *ignoring* grouping (braces/quotes/backslashes).
/// A fast over-estimate mirroring `TclMaxListLength` (`tclUtil.c`) — used to
/// decide error phrasing, never as an actual element count (`{a b}` counts 2).
#[must_use]
pub fn max_list_length(s: &str) -> usize {
    let bytes = s.as_bytes();
    let Some(&first) = bytes.first() else {
        return 0; // empty string: no elements
    };
    // No element precedes leading whitespace.
    let mut count = usize::from(!is_list_space(first));
    let mut i = 0;
    while i < bytes.len() {
        if is_list_space(bytes[i]) {
            count += 1; // a whitespace run starts a (possible) new element
            while i < bytes.len() && is_list_space(bytes[i]) {
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    // No element follows trailing whitespace.
    count - usize::from(is_list_space(bytes[bytes.len() - 1]))
}

/// Describe a value that failed a numeric/boolean coercion, for the
/// `expected <type> but got <here>` error. Matches C (`tclStrToD.c` `formaterr`
/// and `tclObj.c` `TclSetBooleanFromAny`): a multi-token, well-formed list is
/// reported as `a list`; any other value is quoted, truncated to 50 bytes with
/// no ellipsis (`Tcl_AppendLimitedToObj(..., 50, "")`).
#[must_use]
pub fn describe_bad_value(s: &str) -> String {
    if max_list_length(s) > 1 && split_list(s).is_ok() {
        "a list".to_string()
    } else {
        let mut end = s.len().min(50);
        while !s.is_char_boundary(end) {
            end -= 1;
        }
        format!("\"{}\"", &s[..end])
    }
}

// Join — `Tcl_Merge` / `Tcl_ScanElement` + `Tcl_ConvertElement`.

/// How a value must be quoted to appear as one Tcl list element. Mirrors the
/// `CONVERT_*` modes of `TclConvertElement` (tclUtil.c, COMPAT path).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Quote {
    /// `CONVERT_NONE` — copy verbatim.
    None,
    /// `CONVERT_BRACE` — wrap in `{...}`.
    Brace,
    /// `CONVERT_ESCAPE` — backslash-escape every special byte, braces included.
    Escape,
    /// `CONVERT_MASK` — backslash-escape special bytes but leave braces bare
    /// (chosen when quoting is driven solely by `]` / `"`).
    Mask,
}

/// Decide the quoting for `s` as a list element. `leading_hash_unsafe` is the
/// `#`-could-start-a-comment case (true for the first element of a list / a
/// command word; the `TCL_DONT_QUOTE_HASH` inverse).
fn scan_element(s: &str, leading_hash_unsafe: bool) -> Quote {
    if s.is_empty() {
        return Quote::Brace; // the empty element renders as `{}`.
    }
    let b = s.as_bytes();
    // Follows `TclScanElement` (tclUtil.c, `COMPAT 1` — the form Tcl 9.0
    // actually ships). Flags:
    //   forbid_none    ⇒ cannot copy verbatim (must brace or escape).
    //   require_escape ⇒ cannot brace-quote (must escape every special byte).
    //   prefer_escape  ⇒ `]` / `"` seen — prefer the brace-free escape (MASK).
    //   prefer_brace   ⇒ `[` / `$` / `;` / space / backslash / leading char.
    // Balanced braces alone never forbid the bare form; only a *leading* `{`/`"`
    // (or leading `#`) and the substitution/space bytes do. *Unbalanced* braces
    // force escaping.
    let mut forbid_none = false;
    let mut require_escape = false;
    let mut prefer_escape = false;
    let mut prefer_brace = false;
    let mut depth: i32 = 0;

    if leading_hash_unsafe && b[0] == b'#' {
        prefer_brace = true;
    }
    if b[0] == b'{' || b[0] == b'"' {
        forbid_none = true;
        prefer_brace = true;
    }

    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth < 0 {
                    require_escape = true; // unbalanced ⇒ cannot brace.
                }
            }
            b']' | b'"' => {
                forbid_none = true;
                prefer_escape = true;
            }
            b'[' | b'$' | b';' | b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c => {
                // Substitution / terminator / whitespace bytes: cannot copy
                // verbatim, and brace quoting is the preferred protection.
                forbid_none = true;
                prefer_brace = true;
            }
            b'\\' => {
                if i + 1 >= b.len() {
                    require_escape = true; // trailing backslash.
                } else if b[i + 1] == b'\n' {
                    require_escape = true; // `\<newline>` cannot be braced.
                    i += 2;
                    continue;
                } else {
                    forbid_none = true;
                    prefer_brace = true;
                    // `\{` / `\}` / `\\` consume the next byte so it does not
                    // perturb the brace-nesting count.
                    if matches!(b[i + 1], b'{' | b'}' | b'\\') {
                        i += 2;
                        continue;
                    }
                }
            }
            _ => {}
        }
        i += 1;
    }
    if depth > 0 {
        require_escape = true; // unbalanced open brace.
    }

    if require_escape {
        Quote::Escape
    } else if forbid_none {
        if prefer_escape && !prefer_brace {
            Quote::Mask
        } else {
            Quote::Brace
        }
    } else {
        Quote::None
    }
}

fn convert_element(s: &str, quote: Quote, leading_hash_unsafe: bool, out: &mut String) {
    // `TclConvertElement`: a leading `#` is forced into brace quoting unless we
    // are already in full-escape mode (where it becomes `\#`).
    let lead_hash = leading_hash_unsafe && s.starts_with('#');
    let quote = if lead_hash && quote != Quote::Escape {
        Quote::Brace
    } else {
        quote
    };
    match quote {
        Quote::None => out.push_str(s),
        Quote::Brace => {
            out.push('{');
            out.push_str(s);
            out.push('}');
        }
        Quote::Escape | Quote::Mask => {
            // MASK mode leaves balanced braces bare; ESCAPE escapes them too.
            let escape_braces = quote == Quote::Escape;
            for (i, ch) in s.char_indices() {
                match ch {
                    '#' if i == 0 && leading_hash_unsafe => out.push_str("\\#"),
                    '{' | '}' => {
                        if escape_braces {
                            out.push('\\');
                        }
                        out.push(ch);
                    }
                    '[' | ']' | '$' | ';' | '\\' | '"' | ' ' => {
                        out.push('\\');
                        out.push(ch);
                    }
                    '\n' => out.push_str("\\n"),
                    '\t' => out.push_str("\\t"),
                    '\r' => out.push_str("\\r"),
                    '\x0b' => out.push_str("\\v"),
                    '\x0c' => out.push_str("\\f"),
                    _ => out.push(ch),
                }
            }
        }
    }
}

/// Quote a single value as a Tcl list element (`Tcl_ScanElement` +
/// `Tcl_ConvertElement`). The standalone form treats a leading `#` as unsafe.
#[must_use]
pub fn list_element(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    convert_element(s, scan_element(s, true), true, &mut out);
    out
}

/// Join values into a Tcl list string (`Tcl_Merge`). Only the first element's
/// leading `#` is comment-unsafe.
#[must_use]
pub fn join_list<I, S>(elems: I) -> String
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut out = String::new();
    for (idx, e) in elems.into_iter().enumerate() {
        if idx != 0 {
            out.push(' ');
        }
        let s = e.as_ref();
        let hash_unsafe = idx == 0;
        convert_element(s, scan_element(s, hash_unsafe), hash_unsafe, &mut out);
    }
    out
}

/// Re-render `s` as a canonical Tcl list: the same elements, separated by
/// exactly one space, each re-quoted by the shared list-element encoder.
///
/// This is `Tcl_SplitList` followed by `Tcl_Merge` — the only way to
/// re-space a list without changing what it *is*. A hand-rolled scan that
/// splits on whitespace and balanced braces gets the escaped forms wrong:
/// `a\ b` is **one** element (the escaped space is data), `{a b}` is one
/// element whose braces are structure, and a `"a b"` element is one element
/// too. Losing any of those changes the list's length, which for a `proc`
/// parameter list is a change of arity (issue #1196).
///
/// Returns `Err` for input that is not a well-formed list (an unmatched brace
/// or quote, junk after a closing delimiter). A caller that is reformatting
/// source must then preserve the original text verbatim rather than guess.
///
/// Note that Tcl's `\<newline>` line-continuation collapse happens in a
/// *pre-pass* over the script, before any word is list-parsed — even inside
/// braces. A caller holding raw source text must run
/// [`crate::backslash::collapse_brace_continuations_str`] over it first, or
/// the backslash will be read as escaping the newline *inside* an element.
///
/// ```
/// use tcl_syntax::list::normalise_spacing;
/// assert_eq!(normalise_spacing("a    b   c").unwrap(), "a b c");
/// // Structure is preserved, not flattened.
/// assert_eq!(normalise_spacing("a  {b 1}  c").unwrap(), "a {b 1} c");
/// assert_eq!(normalise_spacing(r"a\ b  c").unwrap(), r"{a b} c");
/// assert!(normalise_spacing("a {b").is_err());
/// ```
pub fn normalise_spacing(s: &str) -> Result<String, ListError> {
    Ok(join_list(split_list(s)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn split(s: &str) -> Vec<String> {
        split_list(s)
            .unwrap()
            .into_iter()
            .map(Cow::into_owned)
            .collect()
    }

    #[test]
    fn normalise_spacing_preserves_element_count_and_structure() {
        // TP — plain whitespace runs collapse.
        assert_eq!(normalise_spacing("a    b\t\tc").unwrap(), "a b c");
        // TP — a braced element keeps its braces (one element, not two).
        assert_eq!(normalise_spacing("a   {b 1}   c").unwrap(), "a {b 1} c");
        // TP — an escaped space is data: one element, re-quoted as a brace.
        assert_eq!(normalise_spacing(r"a\ b c").unwrap(), r"{a b} c");
        // TP — a quoted element is one element, re-quoted canonically.
        assert_eq!(normalise_spacing(r#""a b" c"#).unwrap(), "{a b} c");
        // TN — malformed input has no canonical rendering.
        assert!(normalise_spacing("a {b").is_err());
        assert!(normalise_spacing(r#"a "b"#).is_err());
        // FP guard — empty and whitespace-only lists stay empty.
        assert_eq!(normalise_spacing("").unwrap(), "");
        assert_eq!(normalise_spacing("  \n ").unwrap(), "");
    }

    #[test]
    fn normalise_spacing_is_idempotent() {
        for src in [
            "a b c",
            "a {b 1} c",
            r"a\ b c",
            "{} a",
            "args",
            "{a {b c} d}",
        ] {
            let once = normalise_spacing(src).expect("well-formed");
            let twice = normalise_spacing(&once).expect("well-formed");
            assert_eq!(once, twice, "not idempotent for {src:?}");
        }
    }

    #[test]
    fn plain_words() {
        assert_eq!(split("a b c"), ["a", "b", "c"]);
        assert_eq!(split("   "), Vec::<String>::new());
        assert_eq!(split(""), Vec::<String>::new());
    }

    #[test]
    fn whitespace_is_any_list_space() {
        // newline / tab / FF / VT are list separators (unlike command syntax).
        assert_eq!(split("x\ny\tz"), ["x", "y", "z"]);
        assert_eq!(split("a\u{0b}b\u{0c}c"), ["a", "b", "c"]);
    }

    #[test]
    fn braced_elements_are_verbatim() {
        assert_eq!(split("{a b} c"), ["a b", "c"]);
        assert_eq!(split("{a {b c} d}"), ["a {b c} d"]); // nested braces kept
        assert_eq!(split("{a\\nb}"), ["a\\nb"]); // no collapse inside braces
        assert_eq!(split("{}"), [""]); // empty braced element
    }

    #[test]
    fn quoted_and_bare_collapse_backslashes() {
        assert_eq!(split("c\\td"), ["c\td"]);
        assert_eq!(split("\"a b\" c"), ["a b", "c"]);
        assert_eq!(split("\"a\\tb\""), ["a\tb"]);
        // a backslash escapes a would-be terminator
        assert_eq!(split("a\\ b"), ["a b"]);
        assert_eq!(split("\"a\\\"b\""), ["a\"b"]);
    }

    #[test]
    fn errors() {
        assert_eq!(split_list("{unmatched"), Err(ListError::UnmatchedBrace));
        assert_eq!(split_list("\"unmatched"), Err(ListError::UnmatchedQuote));
        assert_eq!(split_list("{a}b"), Err(ListError::BraceFollowedByJunk));
        assert_eq!(split_list("\"a\"b"), Err(ListError::QuoteFollowedByJunk));
    }

    #[test]
    fn backslash_newline_continuation_is_one_element() {
        // `\<newline>` is a line continuation that absorbs the
        // following run of spaces/tabs, so `a\<newline> b` is ONE element, not
        // two — the whitespace after the continuation is part of the element.
        assert_eq!(split_list_raw("a\\\n b").unwrap().len(), 1);
        assert_eq!(split_list_raw("a\\\n\t  b").unwrap().len(), 1);
        // Decoded, the continuation + whitespace collapses to a single space.
        assert_eq!(split_list("a\\\n b").unwrap(), ["a b"]);
        // FP-guard: a real (unescaped) newline/space still separates elements.
        assert_eq!(split_list_raw("a\n b").unwrap().len(), 2);
        assert_eq!(split_list_raw("a b").unwrap().len(), 2);
    }

    #[test]
    fn split_list_raw_keeps_backslashes() {
        // Raw split strips delimiters but does NOT decode backslashes — the
        // element text is verbatim, unlike `split_list`.
        assert_eq!(split_list_raw("a\\ b").unwrap(), ["a\\ b"]); // one element, undecoded
        assert_eq!(split_list_raw("c\\td").unwrap(), ["c\\td"]); // `\t` NOT a tab
        assert_eq!(split_list_raw("{a b} c").unwrap(), ["a b", "c"]);
        assert_eq!(split_list_raw("\"a\\tb\"").unwrap(), ["a\\tb"]);
        // Same error set as `split_list`.
        assert_eq!(split_list_raw("{unmatched"), Err(ListError::UnmatchedBrace));
    }

    #[test]
    fn lenient_variants_recover_on_malformed_tail() {
        // A malformed tail yields the elements parsed before the error, not Err.
        assert_eq!(
            split_list_lenient("a b {unmatched")
                .into_iter()
                .map(Cow::into_owned)
                .collect::<Vec<_>>(),
            ["a", "b"]
        );
        assert_eq!(split_list_raw_lenient("a b {unmatched"), ["a", "b"]);
        // Well-formed input matches the strict variants (raw stays undecoded).
        assert_eq!(split_list_raw_lenient("a\\ b"), ["a\\ b"]);
        assert_eq!(
            split_list_lenient("a\\ b")
                .into_iter()
                .map(Cow::into_owned)
                .collect::<Vec<_>>(),
            ["a b"] // decoded: escaped space, one element
        );
    }

    #[test]
    fn literal_flag_borrows() {
        // A braced element and a backslash-free bare element are literal borrows.
        let s = "{a b} cd";
        let e0 = find_element(s, 0).unwrap().unwrap();
        assert!(e0.literal);
        let e1 = find_element(s, e0.next).unwrap().unwrap();
        assert!(e1.literal);
        // A bare element with a backslash is not literal (needs collapse).
        let e = find_element("a\\tb", 0).unwrap().unwrap();
        assert!(!e.literal);
    }

    #[test]
    fn split_borrows_when_literal() {
        let s = "alpha beta";
        let v = split_list(s).unwrap();
        assert!(matches!(v[0], Cow::Borrowed(_)));
        assert!(matches!(v[1], Cow::Borrowed(_)));
    }

    #[test]
    fn join_basic() {
        assert_eq!(join_list(["a", "b", "c"]), "a b c");
        assert_eq!(join_list(["a b", "c"]), "{a b} c");
        assert_eq!(join_list([""]), "{}");
        assert_eq!(join_list(["a", ""]), "a {}");
    }

    #[test]
    fn join_quoting_choices() {
        // unbalanced brace ⇒ escape mode
        assert_eq!(join_list(["a}b"]), "a\\}b");
        // trailing backslash ⇒ escape mode
        assert_eq!(join_list(["a\\"]), "a\\\\");
        // balanced braces ⇒ brace mode
        assert_eq!(join_list(["a {b} c"]), "{a {b} c}");
        // leading # only quoted for the first element; brace mode is preferred
        assert_eq!(join_list(["#c", "#d"]), "{#c} #d");
        // a leading # that cannot be braced (unbalanced) escapes instead
        assert_eq!(join_list(["#a}b"]), "\\#a\\}b");
        // whitespace/specials force quoting
        assert_eq!(join_list(["a;b"]), "{a;b}");
    }

    #[test]
    fn round_trip() {
        for case in [
            vec!["a", "b", "c"],
            vec!["a b", "c d"],
            vec!["", "x", ""],
            vec!["a}b", "c{d"],
            vec!["a\\b", "x\ty"],
            vec!["#hash", "normal"],
        ] {
            let joined = join_list(&case);
            let back = split(&joined);
            assert_eq!(back, case, "round-trip failed for {case:?} -> {joined:?}");
        }
    }

    /// `list_element` must match C Tcl 9.0's `[list $e]` exactly (COMPAT path of
    /// `TclScanElement`/`TclConvertElement`). Each pair is `(value, rendered)`
    /// captured from `tclsh9.0`. The key cases: balanced braces stay bare
    /// (`b{}`), unbalanced braces escape (`a{}}`/`c}`), `]`/`"` use brace-free
    /// escaping (`a]b`→`a\]b`, `a"b`→`a\"b`) while `[`/`$`/`;` prefer braces
    /// (`a[b`→`{a[b}`, `$x`→`{$x}`), and a leading `#` braces (`{#}`).
    #[test]
    fn list_element_matches_tcl9() {
        for (value, want) in [
            ("b{}", "b{}"),
            ("a{}}", "a\\{\\}\\}"),
            ("c}", "c\\}"),
            ("a{b}c", "a{b}c"),
            ("a{b", "a\\{b"),
            ("{}", "{{}}"),
            ("#", "{#}"),
            ("a\\{b", "{a\\{b}"),
            ("abc", "abc"),
            ("$x", "{$x}"),
            ("[x]", "{[x]}"),
            ("hello world", "{hello world}"),
            ("a;b", "{a;b}"),
            ("x}y", "x\\}y"),
            ("a}}b", "a\\}\\}b"),
            ("a]b", "a\\]b"),
            ("a[b", "{a[b}"),
            ("a\"b", "a\\\"b"),
        ] {
            assert_eq!(list_element(value), want, "list_element({value:?})");
        }
    }

    #[test]
    fn max_list_length_counts_whitespace_runs() {
        // Empty / blank.
        assert_eq!(max_list_length(""), 0);
        assert_eq!(max_list_length("   "), 0);
        // Leading/trailing whitespace does not add elements.
        assert_eq!(max_list_length("a"), 1);
        assert_eq!(max_list_length("  ++1  "), 1);
        // A whitespace run is one separator regardless of width.
        assert_eq!(max_list_length("a   b"), 2);
        assert_eq!(max_list_length("- +1"), 2);
        // Grouping/backslashes are *ignored* — this is an over-estimate, so a
        // single braced/escaped element with inner space still counts > 1.
        assert_eq!(max_list_length("{a b}"), 2);
        assert_eq!(max_list_length("a\\ b"), 2);
        assert_eq!(max_list_length("a b c"), 3);
    }

    #[test]
    fn describe_bad_value_matches_c() {
        // Multi-token well-formed lists report as "a list" (tclStrToD.c formaterr).
        assert_eq!(describe_bad_value("- +1"), "a list");
        assert_eq!(describe_bad_value("{a b}"), "a list");
        assert_eq!(describe_bad_value("a\\ b"), "a list");
        assert_eq!(describe_bad_value("a b c"), "a list");
        // Single tokens are quoted verbatim.
        assert_eq!(describe_bad_value("++1"), "\"++1\"");
        assert_eq!(describe_bad_value("--1.0"), "\"--1.0\"");
        assert_eq!(describe_bad_value("  ++1  "), "\"  ++1  \"");
        // A multi-token string that is *not* a valid list falls back to quoting.
        assert_eq!(describe_bad_value("{a b"), "\"{a b\"");
        // The quoted form truncates to 50 bytes, no ellipsis.
        let long = "x".repeat(60);
        assert_eq!(describe_bad_value(&long), format!("\"{}\"", "x".repeat(50)));
    }
}
