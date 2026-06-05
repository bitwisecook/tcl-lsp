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

use tcl_lexer::{Lexer, SourceMap, Token, TokenType};

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

/// Advance past one balanced `[...]` command substitution. `pos` must index the
/// `[`; returns the offset just past the matching `]`. `\<any>` escapes a
/// bracket. Used by [`scan_parts`].
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

// ---------------------------------------------------------------------------
// Command/word parsing — LOWERED from the canonical `tcl-lexer` token stream
// (the "parse once" convergence: one scanner shared with the LSP/compiler). The
// hard edges (`{*}` prefix, `#`-comment-in-command-position, brace/quote/bracket
// nesting, `$arr(idx)`, line continuation) live in `tcl-lexer`; here we only map
// its tokens into the eval `Command`/`WordPart` model. `scan_parts` remains for
// `subst` (converged next); `split_list` now delegates to the shared
// `tcl_syntax::list` crate.
// ---------------------------------------------------------------------------

/// Byte slice of a token's *content* in the source, delimiter-stripped.
///
/// Delegates to `tcl-lexer`'s [`SourceMap::token_text`] — the **one place** that
/// encodes the "span covers the full token, content strips the wrappers"
/// convention (the leading `$`/`${`/`[`/`{`/`"` via `content_offset`, the
/// trailing `}`/`]`/`"` of the degenerate empty forms, and the zero-content
/// quote-marker `Esc` tokens the scanner emits at a `"…$`/`"…[` boundary). A
/// naive `content_offset..span.end()` range gets all three edge cases wrong, so
/// we reuse the canonical helper and recover the byte range from the returned
/// sub-slice (it borrows the same source buffer as `src`).
fn token_content<'s>(sm: &SourceMap<'s>, src: &'s [u8], t: Token) -> &'s [u8] {
    let txt = sm.token_text(t);
    // The empty-clamp cases return a `&'static ""` (not a sub-slice of the
    // source), so its pointer is unrelated to `src` — short-circuit before the
    // pointer arithmetic below, which would otherwise underflow.
    if txt.is_empty() {
        return b"";
    }
    let start = txt.as_ptr() as usize - src.as_ptr() as usize;
    &src[start..start + txt.len()]
}

/// Parse a whole script into commands via `tcl-lexer`. Empty commands (blank
/// lines, comments) are dropped. Non-UTF-8 input or a lex error yields no
/// commands (the UTF-8 internal-rep invariant; richer parse-error surfacing is
/// tracked with the convergence).
pub fn parse_script(src: &[u8]) -> Vec<Command<'_>> {
    let Ok(s) = std::str::from_utf8(src) else {
        return Vec::new();
    };
    let toks = match Lexer::new(s).tokenise_all() {
        Ok(t) => t,
        Err(_) => return Vec::new(),
    };
    let sm = SourceMap::new(s);
    let mut cmds = Vec::new();
    let mut words: Vec<Word> = Vec::new();
    let mut word_toks: Vec<Token> = Vec::new();
    let mut expand = false;
    for t in toks {
        match t.kind {
            TokenType::Sep => flush_word(&sm, src, &mut words, &mut word_toks, &mut expand),
            TokenType::Comment => {} // a comment is an empty command
            TokenType::Expand => expand = true, // the next word is expanded
            TokenType::Eol | TokenType::Eof => {
                flush_word(&sm, src, &mut words, &mut word_toks, &mut expand);
                if !words.is_empty() {
                    cmds.push(Command {
                        words: core::mem::take(&mut words),
                        next: t.span.end() as usize,
                    });
                }
            }
            _ => word_toks.push(t), // Esc / Str / Cmd / Var → part of the word
        }
    }
    cmds
}

/// Parse the first non-empty command at/after `pos` (for callers that resume by
/// offset; the eval loop uses [`parse_script`]).
pub fn parse_command(src: &[u8], pos: usize) -> Command<'_> {
    let mut cmds = parse_script(&src[pos..]);
    if cmds.is_empty() {
        return Command {
            words: Vec::new(),
            next: src.len(),
        };
    }
    let mut c = cmds.remove(0);
    c.next += pos; // make the resume offset absolute
    c
}

fn flush_word<'s>(
    sm: &SourceMap<'s>,
    src: &'s [u8],
    words: &mut Vec<Word<'s>>,
    word_toks: &mut Vec<Token>,
    expand: &mut bool,
) {
    if word_toks.is_empty() {
        *expand = false; // a stray `{*}` with no following word (shouldn't happen)
        return;
    }
    words.push(build_word(sm, src, word_toks, *expand));
    word_toks.clear();
    *expand = false;
}

fn build_word<'s>(sm: &SourceMap<'s>, src: &'s [u8], toks: &[Token], expand: bool) -> Word<'s> {
    // A single braced token is a literal word (no substitution).
    if toks.len() == 1 && toks[0].kind == TokenType::Str {
        return Word {
            kind: WordKind::Braced,
            expand,
            body: WordBody::Literal(token_content(sm, src, toks[0])),
        };
    }
    // Quoted iff the word opens with `"`. `in_quote` is unreliable for this (the
    // scanner clears it on the *last* token of a quoted word and never sets it on
    // a single-token quoted word), so key off the opening source byte instead —
    // for a quoted word the first token's span always starts at the `"`.
    let kind = if src.get(toks[0].span.start() as usize) == Some(&b'"') {
        WordKind::Quoted
    } else {
        WordKind::Bare
    };
    let mut parts: Vec<WordPart> = Vec::new();
    for &t in toks {
        let bytes = token_content(sm, src, t);
        match t.kind {
            TokenType::Esc => split_esc(bytes, &mut parts),
            TokenType::Str => parts.push(WordPart::Text(bytes)), // braced fragment mid-word (rare)
            TokenType::Var => parts.push(WordPart::Variable(parse_var_ref(
                bytes,
                t.content_offset == 2,
            ))),
            TokenType::Cmd => parts.push(WordPart::Command(bytes)),
            _ => {}
        }
    }
    // Collapse to a borrowed `Literal` (the SIMPLE_WORD fast path): a lone `Text`
    // covers `plainword` and the no-subst quoted form `"hi there"`; an empty part
    // list covers the empty quoted word `""`.
    match parts.as_slice() {
        [WordPart::Text(b)] => Word {
            kind,
            expand,
            body: WordBody::Literal(b),
        },
        [] => Word {
            kind,
            expand,
            body: WordBody::Literal(b""),
        },
        _ => Word {
            kind,
            expand,
            body: WordBody::Parts(parts),
        },
    }
}

/// Split an `Esc` token's content into `Text`/`Backslash` parts (the `$`/`[`
/// substitutions are already separate tokens; only backslashes remain). A
/// trailing lone `\` stays in the `Text` run.
fn split_esc<'s>(bytes: &'s [u8], parts: &mut Vec<WordPart<'s>>) {
    let mut lit_start = 0;
    let mut i = 0;
    let mut buf = [0u8; 4];
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() {
            if i > lit_start {
                parts.push(WordPart::Text(&bytes[lit_start..i]));
            }
            let (next, _) = crate::bs::consume_one(bytes, i + 1, &mut buf);
            parts.push(WordPart::Backslash(&bytes[i..next]));
            i = next;
            lit_start = i;
        } else {
            i += 1;
        }
    }
    if bytes.len() > lit_start {
        parts.push(WordPart::Text(&bytes[lit_start..]));
    }
}

/// Parse a `Var` token's content into name + optional array index. `braced`
/// (the `${name}` form) suppresses index parsing (the whole content is the
/// name); for `$arr(idx)` the index is itself substituted.
fn parse_var_ref(bytes: &[u8], braced: bool) -> VarRef<'_> {
    if !braced && bytes.last() == Some(&b')') {
        if let Some(open) = bytes.iter().position(|&c| c == b'(') {
            let name = &bytes[..open];
            let idx = &bytes[open + 1..bytes.len() - 1];
            let index = match scan_parts(idx, true, true, true) {
                WordBody::Literal(b) => vec![WordPart::Text(b)],
                WordBody::Parts(p) => p,
            };
            return VarRef {
                name,
                index: Some(index),
            };
        }
    }
    VarRef {
        name: bytes,
        index: None,
    }
}

// ---------------------------------------------------------------------------
// List parsing — `Tcl_SplitList` (the primitive `{*}` expansion + the list
// value type need). CONVERGED onto the shared [`tcl_syntax::list`] crate (the
// canonical `FindElement` grammar + `backslash_subst` collapse), so the list
// grammar lives in one place for the runtime AND the LSP/compiler. The runtime
// keeps the byte API (`&[u8]` in, owned bytes out); it converts at the boundary
// on the UTF-8-internal-rep invariant.
// ---------------------------------------------------------------------------

/// Why splitting a string as a Tcl list failed. Mirrors
/// [`tcl_syntax::list::ListError`] plus [`ListError::NotUtf8`] for the
/// byte-boundary case (which cannot occur for a well-formed internal string rep,
/// since the runtime upholds UTF-8 internally).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListError {
    /// An unmatched `{` (`unmatched open brace in list`).
    UnmatchedBrace,
    /// An unmatched `"` (`unmatched open quote in list`).
    UnmatchedQuote,
    /// Junk directly after a closing `}` (`list element in braces followed by`).
    BraceFollowedByJunk,
    /// Junk directly after a closing `"` (`list element in quotes followed by`).
    QuoteFollowedByJunk,
    /// The input bytes were not valid UTF-8 (violates the internal-rep invariant).
    NotUtf8,
}

impl From<tcl_syntax::list::ListError> for ListError {
    fn from(e: tcl_syntax::list::ListError) -> Self {
        use tcl_syntax::list::ListError as L;
        match e {
            L::UnmatchedBrace => ListError::UnmatchedBrace,
            L::UnmatchedQuote => ListError::UnmatchedQuote,
            L::BraceFollowedByJunk => ListError::BraceFollowedByJunk,
            L::QuoteFollowedByJunk => ListError::QuoteFollowedByJunk,
        }
    }
}

/// Split `src` into its Tcl list element *values* (owned): `{braced}` elements
/// verbatim, bare/`"quoted"` elements with backslash escapes decoded. The
/// `Tcl_SplitList` primitive, delegating to [`tcl_syntax::list::split_list`].
pub fn split_list(src: &[u8]) -> Result<Vec<Vec<u8>>, ListError> {
    let s = core::str::from_utf8(src).map_err(|_| ListError::NotUtf8)?;
    Ok(tcl_syntax::list::split_list(s)?
        .into_iter()
        .map(|c| c.into_owned().into_bytes())
        .collect())
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

    // ---- low-level scanners ----

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

    // The list grammar itself is exhaustively tested in `tcl_syntax::list`;
    // here we only smoke-test the byte-API delegation + error mapping.
    #[test]
    fn split_list_delegates_to_tcl_syntax() {
        assert_eq!(
            split_list(b"{a b} c\\td").unwrap(),
            vec![b"a b".to_vec(), b"c\td".to_vec()]
        );
        assert_eq!(split_list(b"   ").unwrap(), Vec::<Vec<u8>>::new());
        assert_eq!(split_list(b"{unmatched"), Err(ListError::UnmatchedBrace));
    }
}
