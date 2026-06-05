//! Tcl script / word parser (T1.2) — a **re-derived** Rust structure, not a
//! transliteration of `runtime/zig/parse/tcl_parse.zig` (a proof-of-concept).
//!
//! Semantics follow reference Tcl 9.0's `Tcl_ParseCommand` family
//! (`tmp/tcl9.0.3/generic/tclParse.c`); the *representation* is chosen for the
//! Rust consumers (see `rust-runtime-port.md` T1.2 representation-decision).
//!
//! ## The model — a borrow-based enum tree
//!
//! The only consumers are the interpreter-fallback eval loop and the
//! `subst`/`eval` family; both want sum-type dispatch with a literal fast path,
//! not C-style `numComponents` index arithmetic. So a parsed command is:
//!
//! - [`Command`] — the words + the resume offset (`next`).
//! - [`Word`] — its delimiter [`WordKind`], the `{*}` `expand` flag, and a
//!   [`WordBody`].
//! - [`WordBody::Literal`] — no substitution needed (a `{braced}` word, or any
//!   bare/quoted word with no `$` `[` `\\`): the bytes **are** the value. This
//!   is Tcl's `TCL_TOKEN_SIMPLE_WORD` fast path, zero-allocation (a borrow).
//! - [`WordBody::Parts`] — components to substitute and concatenate at eval
//!   time: [`WordPart::Text`] / [`Backslash`](WordPart::Backslash) /
//!   [`Variable`](WordPart::Variable) / [`Command`](WordPart::Command).
//!
//! Everything borrows `&'s [u8]` from the source — zero-copy, and the borrow
//! makes the Zig `parse_cache` stale-slab hazard (memory-management.md MM-B.6)
//! a compile error. The module is `unsafe`-free.
//!
//! [`scan_parts`] (the component decomposer) is shared with [`crate::subst`].

#![forbid(unsafe_code)]

/// How a word was delimited in the source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WordKind {
    /// Unquoted: `foo`, `$x`, `a[b]c`.
    Bare,
    /// Double-quoted: `"a $b"` — substitutions active, quotes stripped.
    Quoted,
    /// Braced: `{a $b}` — pure literal, braces stripped, no substitution.
    Braced,
}

/// One substitution component of a non-literal word (or `subst` input).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WordPart<'s> {
    /// Verbatim bytes.
    Text(&'s [u8]),
    /// A backslash escape — the **full** `\x` span (including the backslash).
    /// Decode with [`crate::bs::decode_span`] / [`crate::bs::consume_one`].
    Backslash(&'s [u8]),
    /// `$name` / `${name}` / `$arr(index)`.
    Variable(VarRef<'s>),
    /// `[...]` command substitution — the inner script (brackets stripped).
    Command(&'s [u8]),
}

/// A variable reference. `name` is the bare/`${}` name (always literal). For an
/// `$arr(index)` reference, `index` holds the index's own components (the index
/// is itself substituted at eval time).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VarRef<'s> {
    pub name: &'s [u8],
    pub index: Option<Vec<WordPart<'s>>>,
}

/// A word's content: either a literal (no substitution) or a component list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WordBody<'s> {
    /// The bytes are the value — no substitution. (`SIMPLE_WORD` fast path.)
    Literal(&'s [u8]),
    /// Components to substitute and concatenate.
    Parts(Vec<WordPart<'s>>),
}

/// One parsed word.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Word<'s> {
    pub kind: WordKind,
    /// Preceded by a `{*}` argument-expansion marker (Tcl 8.5+).
    pub expand: bool,
    pub body: WordBody<'s>,
}

/// One parsed command: its words and where to resume parsing the next command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command<'s> {
    pub words: Vec<Word<'s>>,
    /// Offset to resume at for the following command (past the terminator).
    pub next: usize,
}

// ---------------------------------------------------------------------------
// Low-level scanners — return byte offsets into `src`.
// ---------------------------------------------------------------------------

/// Advance past inter-word whitespace: spaces, tabs, and `\<newline>` line
/// continuations (TIP #9 / `TclParseAllWhiteSpace`). Newline/`;`/CR are command
/// terminators, **not** whitespace, so they stop the skip.
pub fn skip_space(src: &[u8], pos: usize) -> usize {
    let len = src.len();
    let mut p = pos;
    while p < len {
        if src[p] == b' ' || src[p] == b'\t' {
            p += 1;
        } else if src[p] == b'\\' && p + 1 < len && src[p + 1] == b'\n' {
            p += 2;
        } else {
            break;
        }
    }
    p
}

/// A delimited word's interior span `[start, start+len)` plus the offset `end`
/// just past the closing delimiter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub len: usize,
    pub end: usize,
}

/// Parse a `{braced}` word. `pos` must index the opening `{`. `\<any>` inside
/// does not change brace depth (so `\{` / `\}` are inert), matching
/// `TclParseBraces`. An unterminated brace runs to end-of-input without
/// underflow.
pub fn find_braced(src: &[u8], pos: usize) -> Span {
    let len = src.len();
    let start = pos + 1;
    let mut p = start;
    let mut depth: usize = 1;
    while p < len && depth > 0 {
        if src[p] == b'\\' && p + 1 < len {
            p += 2;
            continue;
        }
        if src[p] == b'{' {
            depth += 1;
        } else if src[p] == b'}' {
            depth -= 1;
        }
        if depth == 0 {
            break;
        }
        p += 1;
    }
    // p indexes the closing `}` (depth==0) or len (unterminated).
    let (wlen, end) = if depth == 0 {
        (p - start, p + 1)
    } else {
        (p - start, p)
    };
    Span {
        start,
        len: wlen,
        end,
    }
}

/// Parse a `"quoted"` word. `pos` must index the opening `"`. Backslash escapes
/// any following byte (so `\"` does not close the quote).
pub fn find_quoted(src: &[u8], pos: usize) -> Span {
    let len = src.len();
    let start = pos + 1;
    let mut p = start;
    while p < len && src[p] != b'"' {
        if src[p] == b'\\' && p + 1 < len {
            p += 2;
        } else {
            p += 1;
        }
    }
    let wlen = p - start;
    let end = if p < len { p + 1 } else { p };
    Span {
        start,
        len: wlen,
        end,
    }
}

/// Find the end of a bare (unquoted) word: it terminates on top-level
/// whitespace / `;` / newline, but keeps nested `[...]` and `${...}` together
/// (splitting inside them would truncate the inner command/var). A `\<newline>`
/// ends the word (line continuation acts as a separator); other `\x` escapes
/// stay in the word.
pub fn find_bare_end(src: &[u8], pos: usize) -> usize {
    let len = src.len();
    let mut p = pos;
    while p < len {
        let c = src[p];
        if c == b' ' || c == b'\t' || c == b'\n' || c == b';' || c == b'\r' {
            break;
        }
        if c == b'\\' && p + 1 < len {
            if src[p + 1] == b'\n' {
                break;
            }
            p += 2;
        } else if c == b'[' {
            p = skip_command_subst(src, p);
        } else if c == b'$' && p + 1 < len && src[p + 1] == b'{' {
            p += 2;
            while p < len && src[p] != b'}' {
                p += 1;
            }
            if p < len {
                p += 1;
            }
        } else {
            p += 1;
        }
    }
    p
}

/// Advance past one balanced `[...]` command substitution. `pos` must index the
/// `[`; returns the offset just past the matching `]`. `\<any>` escapes a
/// bracket. Shared by [`find_bare_end`] and [`scan_parts`].
pub fn skip_command_subst(src: &[u8], pos: usize) -> usize {
    let len = src.len();
    let mut p = pos + 1;
    let mut depth: usize = 1;
    while p < len && depth > 0 {
        if src[p] == b'\\' && p + 1 < len {
            p += 2;
            continue;
        }
        if src[p] == b'[' {
            depth += 1;
        } else if src[p] == b']' {
            depth -= 1;
        }
        p += 1;
    }
    p
}

// ---------------------------------------------------------------------------
// Component scanner — the shared parse-level decomposition (also used by subst).
// ---------------------------------------------------------------------------

/// Is `c` a valid first byte of an unbraced `$name` variable reference?
/// Letters, digits, underscore, or `:` (namespace separator). Crucially, a `$`
/// **not** followed by one of these (or `{`) is a literal `$` in Tcl
/// (`tclParse.c` `Tcl_ParseVarName`) — the Zig PoC mishandled this.
fn is_var_name_byte(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_' || c == b':'
}

/// Scan an `$name` identifier (already past the `$`), returning the end offset.
/// Accepts alphanumerics / `_` and `::` namespace separators.
fn scan_var_name(src: &[u8], start: usize) -> usize {
    let len = src.len();
    let mut p = start;
    while p < len {
        let c = src[p];
        if c.is_ascii_alphanumeric() || c == b'_' {
            p += 1;
        } else if c == b':' && p + 1 < len && src[p + 1] == b':' {
            p += 2;
        } else {
            break;
        }
    }
    p
}

/// Decompose a span into substitution components, with each substitution kind
/// independently enabled (`do_vars`/`do_cmds`/`do_bs` ↔ `subst`'s
/// `-novariables`/`-nocommands`/`-nobackslashes`). Returns [`WordBody::Literal`]
/// when no enabled substitution actually occurs (the borrow fast path).
///
/// Shared by the word parser (bare/quoted words: all three enabled) and
/// [`crate::subst`]. Does **not** evaluate — `Variable`/`Command` parts carry
/// spans for the eval loop (T1.3/T1.4) to resolve.
pub fn scan_parts(src: &[u8], do_vars: bool, do_cmds: bool, do_bs: bool) -> WordBody<'_> {
    let len = src.len();
    let triggered = src
        .iter()
        .any(|&c| (do_vars && c == b'$') || (do_cmds && c == b'[') || (do_bs && c == b'\\'));
    if !triggered {
        return WordBody::Literal(src);
    }

    let mut parts: Vec<WordPart> = Vec::new();
    let mut lit_start = 0usize;
    let mut i = 0usize;

    while i < len {
        let c = src[i];
        if do_vars
            && c == b'$'
            && i + 1 < len
            && (src[i + 1] == b'{' || is_var_name_byte(src[i + 1]))
        {
            push_text(&mut parts, src, lit_start, i);
            i += 1;
            if src[i] == b'{' {
                // ${name}
                i += 1;
                let ns = i;
                while i < len && src[i] != b'}' {
                    i += 1;
                }
                let ne = i;
                if i < len {
                    i += 1; // consume `}`
                }
                parts.push(WordPart::Variable(VarRef {
                    name: &src[ns..ne],
                    index: None,
                }));
            } else {
                // $name  (optionally  $arr(index))
                let ns = i;
                i = scan_var_name(src, i);
                let name = &src[ns..i];
                if i < len && src[i] == b'(' {
                    i += 1;
                    let ks = i;
                    while i < len && src[i] != b')' {
                        i += 1;
                    }
                    let ke = i;
                    if i < len {
                        i += 1; // consume `)`
                    }
                    // The index is itself substituted at eval time.
                    let index = match scan_parts(&src[ks..ke], do_vars, do_cmds, do_bs) {
                        WordBody::Literal(b) => vec![WordPart::Text(b)],
                        WordBody::Parts(p) => p,
                    };
                    parts.push(WordPart::Variable(VarRef {
                        name,
                        index: Some(index),
                    }));
                } else {
                    parts.push(WordPart::Variable(VarRef { name, index: None }));
                }
            }
            lit_start = i;
        } else if do_cmds && c == b'[' {
            push_text(&mut parts, src, lit_start, i);
            let end = skip_command_subst(src, i);
            // inner script = between the brackets; `end` is one past `]`
            let inner_end = if end > i + 1 && src[end - 1] == b']' {
                end - 1
            } else {
                end
            };
            parts.push(WordPart::Command(&src[i + 1..inner_end]));
            i = end;
            lit_start = i;
        } else if do_bs && c == b'\\' && i + 1 < len {
            push_text(&mut parts, src, lit_start, i);
            // Reuse the decoder to find the escape span's end.
            let mut buf = [0u8; 4];
            let (next, _) = crate::bs::consume_one(src, i + 1, &mut buf);
            parts.push(WordPart::Backslash(&src[i..next]));
            i = next;
            lit_start = i;
        } else {
            i += 1;
        }
    }
    push_text(&mut parts, src, lit_start, len);
    WordBody::Parts(parts)
}

fn push_text<'s>(parts: &mut Vec<WordPart<'s>>, src: &'s [u8], start: usize, end: usize) {
    if end > start {
        parts.push(WordPart::Text(&src[start..end]));
    }
}

// ---------------------------------------------------------------------------
// Command parser.
// ---------------------------------------------------------------------------

/// Parse one command starting at `pos`. Skips leading whitespace, blank lines,
/// and `;`; consumes a leading `#` comment as an empty command. The returned
/// [`Command::next`] is where the caller resumes for the following command.
pub fn parse_command(src: &[u8], pos: usize) -> Command<'_> {
    let len = src.len();
    let mut p = pos;

    // Skip leading whitespace + command terminators (+ line continuations).
    while p < len {
        let c = src[p];
        if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == b';' {
            p += 1;
        } else if c == b'\\' && p + 1 < len && src[p + 1] == b'\n' {
            p += 2;
        } else {
            break;
        }
    }

    // A `#` here introduces a comment that runs to end-of-line: empty command.
    if p < len && src[p] == b'#' {
        while p < len && src[p] != b'\n' {
            p += 1;
        }
        if p < len {
            p += 1;
        }
        return Command {
            words: Vec::new(),
            next: p,
        };
    }

    let mut words: Vec<Word> = Vec::new();
    while p < len {
        p = skip_space(src, p);
        if p >= len || src[p] == b'\n' || src[p] == b';' || src[p] == b'\r' {
            if p < len {
                p += 1; // consume the terminator
            }
            break;
        }

        // `{*}` argument-expansion prefix (Tcl 8.5+). It is a prefix **only**
        // when the three chars `{*}` are *immediately* followed (no space) by a
        // non-blank, non-terminator character (`parser-and-aot-interpret-boundary.md`).
        // Otherwise `{*}` is the ordinary braced word whose value is `*`
        // (standalone, or `{*} x` with a space, or `{*}` at end of command).
        let mut expand = false;
        if src[p] == b'{' && p + 2 < len && src[p + 1] == b'*' && src[p + 2] == b'}' {
            let after = p + 3;
            let immediately_followed = after < len
                && !matches!(
                    src[after],
                    b' ' | b'\t' | b'\n' | b'\r' | b';' | 0x0b | 0x0c
                );
            if immediately_followed {
                expand = true;
                p += 3; // strip the prefix; the following word is parsed below
            }
            // else: leave `expand` false and `p` unchanged — `find_braced`
            // below parses `{*}` as a normal braced word (value `*`).
        }

        let word = if src[p] == b'{' {
            let s = find_braced(src, p);
            p = s.end;
            Word {
                kind: WordKind::Braced,
                expand,
                body: WordBody::Literal(&src[s.start..s.start + s.len]),
            }
        } else if src[p] == b'"' {
            let s = find_quoted(src, p);
            p = s.end;
            let span = &src[s.start..s.start + s.len];
            Word {
                kind: WordKind::Quoted,
                expand,
                body: scan_parts(span, true, true, true),
            }
        } else {
            let start = p;
            let end = find_bare_end(src, p);
            p = end;
            let span = &src[start..end];
            Word {
                kind: WordKind::Bare,
                expand,
                body: scan_parts(span, true, true, true),
            }
        };
        words.push(word);
    }

    Command { words, next: p }
}

/// Parse a whole script into commands. Empty commands (blank lines, comments)
/// are dropped so callers see only commands with words.
pub fn parse_script(src: &[u8]) -> Vec<Command<'_>> {
    let mut out = Vec::new();
    let mut p = 0;
    while p < src.len() {
        let cmd = parse_command(src, p);
        // Guard against non-advancement on pathological input.
        if cmd.next <= p {
            break;
        }
        p = cmd.next;
        if !cmd.words.is_empty() {
            out.push(cmd);
        }
    }
    out
}

// ---------------------------------------------------------------------------
// List parsing — `Tcl_SplitList` (the primitive `{*}` expansion and the list
// value type need). Distinct from command parsing: no comments, `;`/newline are
// plain whitespace, and there is no `$`/`[` substitution — only brace/quote
// grouping and (for bare/quoted elements) backslash decoding.
// ---------------------------------------------------------------------------

/// Why splitting a string as a Tcl list failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListError {
    /// An unmatched `{` (`unmatched open brace in list`).
    UnmatchedBrace,
    /// An unmatched `"` (`unmatched open quote in list`).
    UnmatchedQuote,
}

#[inline]
fn is_list_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
}

/// Split `src` into its Tcl list elements, decoding each element's value:
/// `{braced}` elements are taken verbatim (no substitution); bare and
/// `"quoted"` elements have backslash escapes decoded. This is the
/// `Tcl_SplitList` primitive (element *values*, owned).
pub fn split_list(src: &[u8]) -> Result<Vec<Vec<u8>>, ListError> {
    let len = src.len();
    let mut out = Vec::new();
    let mut p = 0;
    loop {
        while p < len && is_list_space(src[p]) {
            p += 1;
        }
        if p >= len {
            break;
        }
        if src[p] == b'{' {
            let s = find_braced(src, p);
            // find_braced runs to end-of-input on an unterminated brace; detect
            // that (the byte before `end` is not the closing `}`).
            if s.end > len || (s.end == len && (len == 0 || src[len - 1] != b'}')) {
                return Err(ListError::UnmatchedBrace);
            }
            out.push(src[s.start..s.start + s.len].to_vec());
            p = s.end;
        } else if src[p] == b'"' {
            let s = find_quoted(src, p);
            if s.end == len && (len == 0 || src[len - 1] != b'"') {
                return Err(ListError::UnmatchedQuote);
            }
            out.push(crate::bs::decode_span(&src[s.start..s.start + s.len]));
            p = s.end;
        } else {
            let start = p;
            while p < len && !is_list_space(src[p]) {
                if src[p] == b'\\' && p + 1 < len {
                    p += 2;
                } else {
                    p += 1;
                }
            }
            out.push(crate::bs::decode_span(&src[start..p]));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lit<'a>(w: &Word<'a>) -> &'a [u8] {
        match w.body {
            WordBody::Literal(b) => b,
            _ => panic!("expected Literal, got {:?}", w.body),
        }
    }
    fn parts<'a>(w: &'a Word<'a>) -> &'a [WordPart<'a>] {
        match &w.body {
            WordBody::Parts(p) => p,
            _ => panic!("expected Parts, got {:?}", w.body),
        }
    }

    // ---- low-level scanners (mirror test_tcl_parse.zig) ----

    #[test]
    fn skip_space_spaces_tabs_continuation() {
        assert_eq!(skip_space(b"  \tabc", 0), 3);
        assert_eq!(skip_space(b"  \\\n  abc", 0), 6); // `\<nl>` folds
        assert_eq!(skip_space(b"\nabc", 0), 0); // newline is a terminator
    }

    #[test]
    fn braced_simple_nested_escaped_unterminated() {
        let s = find_braced(b"{abc}", 0);
        assert_eq!((s.start, s.len, s.end), (1, 3, 5));
        let src = b"{a {b c} d}";
        let s = find_braced(src, 0);
        assert_eq!(&src[s.start..s.start + s.len], b"a {b c} d");
        let src = b"{a\\{b\\}c}";
        let s = find_braced(src, 0);
        assert_eq!(&src[s.start..s.start + s.len], b"a\\{b\\}c");
        // unterminated `{` must not underflow
        let s = find_braced(b"{", 0);
        assert_eq!((s.start, s.len), (1, 0));
    }

    #[test]
    fn quoted_and_backslash_quote() {
        let src = b"\"hello world\" tail";
        let s = find_quoted(src, 0);
        assert_eq!(&src[s.start..s.start + s.len], b"hello world");
        assert_eq!(s.end, 13);
        let src = b"\"a\\\"b\" rest";
        let s = find_quoted(src, 0);
        assert_eq!(&src[s.start..s.start + s.len], b"a\\\"b");
    }

    #[test]
    fn bare_end_keeps_substitutions_together() {
        assert_eq!(find_bare_end(b"foo bar", 0), 3);
        assert_eq!(find_bare_end(b"[clock seconds]xy more", 0), 17);
        assert_eq!(find_bare_end(b"x${name}y rest", 0), 9);
        // `\<nl>` terminates the word
        assert_eq!(find_bare_end(b"foo\\\nbar", 0), 3);
    }

    #[test]
    fn command_subst_balances_and_escapes() {
        assert_eq!(skip_command_subst(b"[a [b] c] tail", 0), 9);
        assert_eq!(skip_command_subst(b"[a\\] b] tail", 0), 7);
    }

    // ---- parse_command ----

    #[test]
    fn two_word_command() {
        let c = parse_command(b"puts hello", 0);
        assert_eq!(c.words.len(), 2);
        assert_eq!(lit(&c.words[0]), b"puts");
        assert_eq!(lit(&c.words[1]), b"hello");
        assert_eq!(c.words[0].kind, WordKind::Bare);
        assert!(!c.words[0].expand);
    }

    #[test]
    fn braced_word_is_literal() {
        let c = parse_command(b"set x {hello world}", 0);
        assert_eq!(c.words.len(), 3);
        assert_eq!(c.words[2].kind, WordKind::Braced);
        assert_eq!(lit(&c.words[2]), b"hello world");
    }

    #[test]
    fn quoted_word_strips_quotes() {
        let c = parse_command(b"puts \"hi there\"", 0);
        assert_eq!(c.words.len(), 2);
        assert_eq!(c.words[1].kind, WordKind::Quoted);
        assert_eq!(lit(&c.words[1]), b"hi there");
    }

    #[test]
    fn semicolon_ends_command() {
        let c = parse_command(b"a b ; c d", 0);
        assert_eq!(c.words.len(), 2);
        assert_eq!(lit(&c.words[0]), b"a");
        assert_eq!(lit(&c.words[1]), b"b");
        // resume past the `;` and parse the next command
        let c2 = parse_command(b"a b ; c d", c.next);
        assert_eq!(lit(&c2.words[0]), b"c");
        assert_eq!(lit(&c2.words[1]), b"d");
    }

    #[test]
    fn comment_is_empty_command() {
        let c = parse_command(b"# this is a comment\n", 0);
        assert!(c.words.is_empty());
    }

    #[test]
    fn expand_marker_sets_flag_and_strips() {
        let c = parse_command(b"foo {*}$args bar", 0);
        assert_eq!(c.words.len(), 3);
        assert_eq!(lit(&c.words[0]), b"foo");
        assert!(c.words[1].expand);
        assert_eq!(
            parts(&c.words[1]),
            &[WordPart::Variable(VarRef {
                name: b"args",
                index: None
            })]
        );
        assert!(!c.words[2].expand);
        assert_eq!(lit(&c.words[2]), b"bar");
    }

    #[test]
    fn expand_only_when_immediately_followed() {
        // `{*}` is a prefix only when immediately (no space) followed by a
        // non-blank, non-terminator char. Otherwise it is the literal word `*`.
        // standalone `{*}` → literal braced word "*", not an empty expansion
        let c = parse_command(b"foo {*}", 0);
        assert_eq!(c.words.len(), 2);
        assert!(!c.words[1].expand);
        assert_eq!(c.words[1].kind, WordKind::Braced);
        assert_eq!(lit(&c.words[1]), b"*");
        // `{*} x` (space after) → literal `{*}` then `x`, NOT expansion
        let c = parse_command(b"foo {*} x", 0);
        assert_eq!(c.words.len(), 3);
        assert!(!c.words[1].expand);
        assert_eq!(lit(&c.words[1]), b"*");
        assert!(!c.words[2].expand);
        assert_eq!(lit(&c.words[2]), b"x");
        // `{*}{a b}` → expansion of the braced word `a b`
        let c = parse_command(b"foo {*}{a b}", 0);
        assert_eq!(c.words.len(), 2);
        assert!(c.words[1].expand);
        assert_eq!(lit(&c.words[1]), b"a b");
    }

    #[test]
    fn line_continuation_joins_logical_line() {
        let c = parse_command(b"cmd a \\\n   b c", 0);
        let got: Vec<&[u8]> = c.words.iter().map(lit).collect();
        assert_eq!(got, vec![&b"cmd"[..], b"a", b"b", b"c"]);
    }

    // ---- component decomposition (what the Zig token tree left as a TODO) ----

    #[test]
    fn bracket_subst_decomposes_in_one_word() {
        let c = parse_command(b"set x [clock seconds]", 0);
        assert_eq!(c.words.len(), 3);
        assert_eq!(parts(&c.words[2]), &[WordPart::Command(b"clock seconds")]);
    }

    #[test]
    fn mixed_text_and_variable() {
        let c = parse_command(b"puts x${name}y", 0);
        assert_eq!(
            parts(&c.words[1]),
            &[
                WordPart::Text(b"x"),
                WordPart::Variable(VarRef {
                    name: b"name",
                    index: None
                }),
                WordPart::Text(b"y"),
            ]
        );
    }

    #[test]
    fn array_ref_index_components() {
        let c = parse_command(b"puts $arr($i)", 0);
        match &c.words[1].body {
            WordBody::Parts(p) => match &p[0] {
                WordPart::Variable(v) => {
                    assert_eq!(v.name, b"arr");
                    assert_eq!(
                        v.index.as_deref(),
                        Some(
                            &[WordPart::Variable(VarRef {
                                name: b"i",
                                index: None
                            })][..]
                        )
                    );
                }
                other => panic!("expected Variable, got {other:?}"),
            },
            other => panic!("expected Parts, got {other:?}"),
        }
    }

    #[test]
    fn literal_word_is_zero_copy() {
        // A word with no $ [ \ is a borrow of the source, not a Vec.
        let src = b"plainword rest";
        let c = parse_command(src, 0);
        match c.words[0].body {
            WordBody::Literal(b) => assert!(std::ptr::eq(b.as_ptr(), src.as_ptr())),
            _ => panic!("expected zero-copy Literal"),
        }
    }

    #[test]
    fn script_drops_empty_commands() {
        let cmds = parse_script(b"# c\nputs hi\n\nset x 1\n");
        assert_eq!(cmds.len(), 2);
        assert_eq!(lit(&cmds[0].words[0]), b"puts");
        assert_eq!(lit(&cmds[1].words[0]), b"set");
    }

    #[test]
    fn split_list_grouping_and_decode() {
        assert_eq!(
            split_list(b"a b c").unwrap(),
            vec![b"a".to_vec(), b"b".to_vec(), b"c".to_vec()]
        );
        // braced elements are verbatim; bare elements decode backslashes
        assert_eq!(
            split_list(b"{a b} c\\td").unwrap(),
            vec![b"a b".to_vec(), b"c\td".to_vec()]
        );
        // newlines are plain whitespace in list context
        assert_eq!(
            split_list(b"x\ny\tz").unwrap(),
            vec![b"x".to_vec(), b"y".to_vec(), b"z".to_vec()]
        );
        // quoted element
        assert_eq!(
            split_list(b"\"a b\" c").unwrap(),
            vec![b"a b".to_vec(), b"c".to_vec()]
        );
        assert_eq!(split_list(b"   ").unwrap(), Vec::<Vec<u8>>::new());
        assert_eq!(split_list(b"{unmatched"), Err(ListError::UnmatchedBrace));
    }
}
