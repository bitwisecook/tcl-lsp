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

//! Tcl script / word parser (T1.2) — a **re-derived** Rust structure.
//!
//! Semantics follow reference Tcl 9.0's `Tcl_ParseCommand` family
//! (`tmp/tcl9.0.3/generic/tclParse.c`); the *representation* is chosen for the
//! Rust consumers.
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
//!   time: [`WordPart::Text`] (escapes already folded in via the shared
//!   `tcl_syntax::backslash` decoder) / [`Variable`](WordPart::Variable) /
//!   [`Command`](WordPart::Command).
//!
//! `Variable`/`Command`/literal-`Text` borrow `&'s [u8]` from the source —
//! zero-copy on the fast path, and the borrow makes the `parse_cache`
//! stale-slab hazard (memory-management.md MM-B.6) a compile error. A `Text`
//! run whose escapes were decoded owns its bytes (`Cow::Owned`). The module is
//! `unsafe`-free.
//!
//! [`scan_parts`] (the component decomposer) is shared with [`crate::subst`].

#![forbid(unsafe_code)]

use std::borrow::Cow;

use tcl_core_types::RecursionLimit;
use tcl_lexer::{EscapeSyntax, Lexer, LexerConfig, SourceMap, Token, TokenType};

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
    /// A resolved literal run: text with its backslash escapes already decoded
    /// (via the canonical [`tcl_syntax::backslash`] decoder) when backslash
    /// substitution is active. Borrows the source when there was nothing to
    /// decode (the fast path); owns the decoded bytes otherwise. There is no
    /// separate "backslash" component — escapes fold into the surrounding run,
    /// so this matches Tcl's `subst` output with one decoder, not two.
    Text(Cow<'s, [u8]>),
    /// `$name` / `${name}` / `$arr(index)`.
    Variable(VarRef<'s>),
    /// `[...]` command substitution — the inner script (brackets stripped).
    Command(&'s [u8]),
    /// A malformed construct C's parser rejects, carrying the error it raises
    /// (currently only [`tcl_lexer::MISSING_CLOSE_BRACE_FOR_VAR`], from an
    /// unterminated `${…}`).
    ///
    /// The scanner is infallible by design — it is shared with the LSP-facing
    /// word parser, which must keep tokenizing broken source — so the error
    /// travels as a *part* and the evaluator raises it when the left-to-right
    /// walk reaches it. That is exactly C's order: in `subst` a command
    /// substitution earlier in the same template has already run and kept its
    /// side effects before the bad `${` is reported (issue #1457).
    ParseError(&'static str),
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
    /// Byte offset of the word's first token in `src` — used to compute the
    /// word's source line (TIP 280 argument-line tracking), e.g. so a method /
    /// proc body defined on a later line than its command reports file-absolute
    /// `info frame` lines.
    pub start: usize,
}

/// One parsed command: its words and where to resume parsing the next command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command<'s> {
    pub words: Vec<Word<'s>>,
    /// Offset to resume at for the following command (past the terminator).
    pub next: usize,
    /// Byte offset of the command's first word in `src` — C's `commandStart`
    /// (after leading whitespace). The 1-based source line of the command is
    /// `1 + count('\n' in src[0..start])` (`TclLogCommandInfo`, the errorInfo
    /// stack-trace line).
    pub start: usize,
    /// Byte offset of the command's terminator (`\n`/`;`) or end-of-script — the
    /// command source slice is `src[start..end]`, which **keeps trailing
    /// whitespace but excludes the terminator** (matches C's logged command
    /// string: `"error boom   "` but not the trailing newline).
    pub end: usize,
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
/// (`tclParse.c` `Tcl_ParseVarName`).
fn is_var_name_byte(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_' || c == b':'
}

/// Scan an `$name` identifier (already past the `$`), returning the end offset.
/// Accepts alphanumerics / `_` and `::` namespace separators.
fn scan_var_name(src: &[u8], start: usize) -> usize {
    tcl_core_types::naming::scan_var_name_end(src, start)
}

/// Cap on `$name(index)` nesting depth [`scan_parts`] will recurse into while
/// parsing an array-index expression's own substitution components. The
/// index-parsing call is self-recursive with no natural bound —
/// `$a($b($c(...)))` recurses one native-stack frame group per `(` — so
/// pathologically deep/generated input (reachable via ordinary `subst`/word
/// substitution, no special syntax needed) could otherwise abort the process
/// with an uncatchable stack overflow (issue #996). Empirically, the same
/// class of unguarded nested-array-index recursion overflowed the native
/// stack (SIGABRT) between depth 100-150 on a 256 KiB stack, still crashing
/// at depth 2000 on a 1 MiB stack (this crate's own sweep; see
/// `tcl_lexer::lexer::MAX_ARRAY_INDEX_DEPTH`, which measured the same
/// construct at the lexer layer and crashed in the same 20,000-25,000 range
/// on a 2 MiB thread). 64 mirrors that constant: far past any array-index
/// nesting real Tcl code uses, comfortably under every measured crash
/// threshold above with room to spare for a smaller WASM host stack.
const MAX_SCAN_PARTS_DEPTH: RecursionLimit = RecursionLimit(64);

/// Decompose a span into substitution components, with each substitution kind
/// independently enabled (`do_vars`/`do_cmds`/`do_bs` ↔ `subst`'s
/// `-novariables`/`-nocommands`/`-nobackslashes`). Returns [`WordBody::Literal`]
/// when no enabled substitution actually occurs (the borrow fast path).
///
/// Shared by the word parser (bare/quoted words: all three enabled) and
/// [`crate::subst`]. Does **not** evaluate — `Variable`/`Command` parts carry
/// spans for the eval loop (T1.3/T1.4) to resolve.
pub fn scan_parts(
    src: &[u8],
    do_vars: bool,
    do_cmds: bool,
    do_bs: bool,
    config: LexerConfig,
) -> WordBody<'_> {
    scan_parts_at_depth(src, do_vars, do_cmds, do_bs, config, 0)
}

/// [`scan_parts`]'s implementation, threading the `$name(index)` nesting
/// `depth` through the self-recursive index-parsing call — see
/// [`MAX_SCAN_PARTS_DEPTH`]. Past the cap, a nested `$name(index)`'s index is
/// no longer itself scanned for substitutions; it is kept as a literal text
/// run instead (mirroring `tcl_lexer::lexer::scan_array_index_body`'s
/// graceful degradation), so pathologically deep input degrades gracefully
/// rather than recursing further.
fn scan_parts_at_depth(
    src: &[u8],
    do_vars: bool,
    do_cmds: bool,
    do_bs: bool,
    config: LexerConfig,
    depth: u32,
) -> WordBody<'_> {
    let escapes = config.escapes;
    let len = src.len();
    let triggered = src
        .iter()
        .any(|&c| (do_vars && c == b'$') || (do_cmds && c == b'[') || (do_bs && c == b'\\'));
    if !triggered {
        return WordBody::Literal(src);
    }
    // Computed once per call (not per-character): whether this call is
    // already at/past the depth cap, so any `$name(index)` found in this
    // span keeps its index as literal text instead of recursing further.
    let past_cap = MAX_SCAN_PARTS_DEPTH.exceeded(depth);

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
            flush_text(&mut parts, src, lit_start, i, do_bs, escapes);
            i += 1;
            if src[i] == b'{' {
                // `${name}` — the close rule is release-specific
                // (`Tcl_ParseVarName`): 8.x ends the name at the first literal
                // `}`, 9.x counts nested braces and skips `\X` pairs, so
                // `subst {${a{b}c}}` reads `a{b` under 8.6 and `a{b}c` under
                // 9.0 (issue #1457). Resolved through the one shared owner.
                i += 1;
                let ns = i;
                match tcl_lexer::braced_var_name_end(src, ns, config.braced_var) {
                    tcl_lexer::BracedVarEnd::Closed(ne) => {
                        i = ne + 1; // consume the `}`
                        parts.push(WordPart::Variable(VarRef {
                            name: &src[ns..ne],
                            index: None,
                        }));
                    }
                    // C raises here rather than reading a name that runs to
                    // end-of-input; the 9.x rule also calls `${a\}` and `${a{b}`
                    // unterminated where 8.x closes them.
                    tcl_lexer::BracedVarEnd::Unterminated => {
                        i = len;
                        parts.push(WordPart::ParseError(tcl_lexer::MISSING_CLOSE_BRACE_FOR_VAR));
                    }
                }
            } else {
                // $name  (optionally  $arr(index))
                let ns = i;
                i = scan_var_name(src, i);
                let name = &src[ns..i];
                if i < len && src[i] == b'(' {
                    let scan =
                        tcl_lexer::scan_array_index(src, i, config.array_index, config.braced_var);
                    let (ks, ke) = match scan.end {
                        tcl_lexer::ArrayIndexEnd::Closed(end) => (i + 1, end - 1),
                        tcl_lexer::ArrayIndexEnd::Unterminated => (i + 1, len),
                    };
                    i = match scan.end {
                        tcl_lexer::ArrayIndexEnd::Closed(end) => end,
                        tcl_lexer::ArrayIndexEnd::Unterminated => len,
                    };
                    if scan.invalid.is_some() {
                        parts.push(WordPart::ParseError(
                            tcl_lexer::INVALID_CHARACTER_IN_ARRAY_INDEX,
                        ));
                        lit_start = i;
                        continue;
                    }
                    // The index is itself substituted at eval time — unless
                    // this call is already past the depth cap, in which case
                    // the index is kept as a literal run instead of recursing.
                    let index = if past_cap {
                        vec![WordPart::Text(Cow::Borrowed(&src[ks..ke]))]
                    } else {
                        match scan_parts_at_depth(
                            &src[ks..ke],
                            do_vars,
                            do_cmds,
                            do_bs,
                            config,
                            depth + 1,
                        ) {
                            WordBody::Literal(b) => vec![WordPart::Text(Cow::Borrowed(b))],
                            WordBody::Parts(p) => p,
                        }
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
            flush_text(&mut parts, src, lit_start, i, do_bs, escapes);
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
            // Escaped byte: skip the `\` and the byte it escapes so an escaped
            // `$`/`[` is not a substitution boundary. Only the immediately
            // following byte matters here (longer escapes' trailing digits are
            // plain run text); the whole run is decoded once at `flush_text`.
            i += 2;
        } else {
            i += 1;
        }
    }
    flush_text(&mut parts, src, lit_start, len, do_bs, escapes);
    WordBody::Parts(parts)
}

/// Push the literal run `src[start..end]` as a [`WordPart::Text`], decoding its
/// backslash escapes via the shared decoder under the emulated release's
/// grammar when `do_bs` (borrowing otherwise).
fn flush_text<'s>(
    parts: &mut Vec<WordPart<'s>>,
    src: &'s [u8],
    start: usize,
    end: usize,
    do_bs: bool,
    escapes: EscapeSyntax,
) {
    if end > start {
        let run = &src[start..end];
        let text = if do_bs {
            tcl_syntax::backslash::decode_bytes_in(run, escapes)
        } else {
            Cow::Borrowed(run)
        };
        parts.push(WordPart::Text(text));
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
///
/// Lexes with the default (Tcl-8.5+) config; a version-pinned interpreter
/// parses through [`parse_script_with_config`] instead so the grammar follows
/// the emulated release (issue #1462).
pub fn parse_script(src: &[u8]) -> Vec<Command<'_>> {
    parse_script_with_config(src, tcl_lexer::LexerConfig::default())
}

/// [`parse_script`] under an explicit dialect [`tcl_lexer::LexerConfig`] —
/// the seam [`crate::interp::Interp`] threads its runtime release's grammar
/// through (issue #1462), so `{*}` expansion is off and the first-close
/// `${…}` rule applies when the interpreter emulates Tcl 8.4.
pub fn parse_script_with_config(src: &[u8], config: tcl_lexer::LexerConfig) -> Vec<Command<'_>> {
    let Ok(s) = std::str::from_utf8(src) else {
        return Vec::new();
    };
    let toks = match Lexer::with_source_map(SourceMap::new(s), config).tokenise_all() {
        Ok(t) => t,
        Err(_) => return Vec::new(),
    };
    let sm = SourceMap::new(s);
    let mut cmds = Vec::new();
    let mut words: Vec<Word> = Vec::new();
    let mut word_toks: Vec<Token> = Vec::new();
    let mut expand = false;
    // The command's first-word offset (`commandStart`), set at the first
    // non-whitespace token of each command; `None` between commands.
    let mut cmd_start: Option<usize> = None;
    for t in toks {
        match t.kind {
            TokenType::Sep => flush_word(&sm, src, &mut words, &mut word_toks, &mut expand, config),
            TokenType::Comment => {} // a comment is an empty command
            TokenType::Expand => {
                expand = true; // the next word is expanded
                cmd_start.get_or_insert(t.span.start() as usize);
            }
            TokenType::Eol | TokenType::Eof => {
                flush_word(&sm, src, &mut words, &mut word_toks, &mut expand, config);
                if !words.is_empty() {
                    cmds.push(Command {
                        words: core::mem::take(&mut words),
                        next: t.span.end() as usize,
                        start: cmd_start.take().unwrap_or(t.span.start() as usize),
                        // The terminator (`\n`/`;`) or EOF position — its span
                        // start is the first byte past the command's content, so
                        // `src[start..end]` keeps trailing whitespace but drops
                        // the terminator.
                        end: t.span.start() as usize,
                    });
                } else {
                    cmd_start = None;
                }
            }
            // Esc / Str / Cmd / Var → part of the word.
            _ => {
                cmd_start.get_or_insert(t.span.start() as usize);
                word_toks.push(t);
            }
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
            start: src.len(),
            end: src.len(),
        };
    }
    let mut c = cmds.remove(0);
    // Make the offsets absolute (they were computed against `src[pos..]`).
    c.next += pos;
    c.start += pos;
    c.end += pos;
    c
}

fn flush_word<'s>(
    sm: &SourceMap<'s>,
    src: &'s [u8],
    words: &mut Vec<Word<'s>>,
    word_toks: &mut Vec<Token>,
    expand: &mut bool,
    config: LexerConfig,
) {
    if word_toks.is_empty() {
        *expand = false; // a stray `{*}` with no following word (shouldn't happen)
        return;
    }
    words.push(build_word(sm, src, word_toks, *expand, config));
    word_toks.clear();
    *expand = false;
}

fn build_word<'s>(
    sm: &SourceMap<'s>,
    src: &'s [u8],
    toks: &[Token],
    expand: bool,
    config: LexerConfig,
) -> Word<'s> {
    let escapes = config.escapes;
    // A single braced token is a literal word (no substitution) — except a
    // backslash-newline line continuation, the one backslash sequence a braced
    // word collapses (to a single space, swallowing following spaces/tabs), as
    // C does in its pre-parse pass. A word with no continuation borrows the
    // source unchanged; one with a continuation owns the collapsed bytes.
    if toks.len() == 1 && toks[0].kind == TokenType::Str {
        let content = token_content(sm, src, toks[0]);
        let body = match tcl_syntax::backslash::collapse_brace_continuations(content) {
            Cow::Borrowed(b) => WordBody::Literal(b),
            Cow::Owned(o) => WordBody::Parts(vec![WordPart::Text(Cow::Owned(o))]),
        };
        return Word {
            kind: WordKind::Braced,
            expand,
            body,
            start: toks[0].span.start() as usize,
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
            // An `Esc` token's content is one literal+backslash run (`$`/`[` are
            // already separate tokens), so decode it in one shot via the shared
            // decoder — no per-escape splitting.
            TokenType::Esc if !bytes.is_empty() => {
                parts.push(WordPart::Text(tcl_syntax::backslash::decode_bytes_in(
                    bytes, escapes,
                )));
            }
            // An empty (quote-marker) `Esc` contributes nothing.
            TokenType::Esc => {}
            // A braced fragment mid-word (rare) is verbatim apart from the
            // backslash-newline line continuation, which collapses to a space.
            TokenType::Str => {
                parts.push(WordPart::Text(
                    tcl_syntax::backslash::collapse_brace_continuations(bytes),
                ));
            }
            TokenType::Var => {
                if array_index_parse_error(bytes, t.content_offset == 2, config) {
                    parts.push(WordPart::ParseError(
                        tcl_lexer::INVALID_CHARACTER_IN_ARRAY_INDEX,
                    ));
                } else {
                    parts.push(WordPart::Variable(parse_var_ref(
                        bytes,
                        t.content_offset == 2,
                        config,
                    )));
                }
            }
            TokenType::Cmd => parts.push(WordPart::Command(bytes)),
            _ => {}
        }
    }
    // SIMPLE_WORD fast path: an empty word, or a lone *borrowed* `Text` (its run
    // had no escapes to decode — `plainword` / the no-subst quoted `"hi there"`),
    // collapses to a borrowed `Literal`. A lone *owned* `Text` (escapes decoded)
    // stays `Parts` — `substitute_word` resolves it through the buffer path.
    if parts.is_empty() {
        return Word {
            kind,
            expand,
            body: WordBody::Literal(b""),
            start: toks[0].span.start() as usize,
        };
    }
    if let [WordPart::Text(Cow::Borrowed(b))] = parts.as_slice() {
        return Word {
            kind,
            expand,
            body: WordBody::Literal(b),
            start: toks[0].span.start() as usize,
        };
    }
    Word {
        kind,
        expand,
        body: WordBody::Parts(parts),
        start: toks[0].span.start() as usize,
    }
}

/// Parse a `Var` token's content into name + optional array index. `braced`
/// (the `${name}` form) suppresses index parsing (the whole content is the
/// name); for `$arr(idx)` the index is itself substituted.
fn parse_var_ref(bytes: &[u8], braced: bool, config: LexerConfig) -> VarRef<'_> {
    if !braced && bytes.last() == Some(&b')') {
        if let Some(open) = bytes.iter().position(|&c| c == b'(') {
            let name = &bytes[..open];
            let idx = &bytes[open + 1..bytes.len() - 1];
            let index = match scan_parts(idx, true, true, true, config) {
                WordBody::Literal(b) => vec![WordPart::Text(Cow::Borrowed(b))],
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

/// Whether a lexer-produced variable token carries an array-index source byte
/// forbidden by the selected release. The script lexer records the same fact
/// as a recovery warning; this runtime parser turns it into an evaluable
/// parse-error part so direct Rust/WASM interpretation cannot bypass #1732.
fn array_index_parse_error(bytes: &[u8], braced: bool, config: LexerConfig) -> bool {
    if braced {
        return false;
    }
    let open = scan_var_name(bytes, 0);
    bytes.get(open) == Some(&b'(')
        && tcl_lexer::scan_array_index(bytes, open, config.array_index, config.braced_var)
            .invalid
            .is_some()
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

impl ListError {
    /// The Tcl error message for this failure — reusing the **shared**
    /// [`tcl_syntax::list::ListError`] strings (one source). The `…FollowedByJunk`
    /// variants are the message *prefix*; byte-exact text appends `"<frag>"
    /// instead of space`, which needs the offending fragment surfaced from the
    /// splitter (tracked follow-up).
    #[must_use]
    pub fn message(self) -> &'static [u8] {
        match self {
            ListError::UnmatchedBrace => tcl_syntax::list::ListError::UnmatchedBrace.message(),
            ListError::UnmatchedQuote => tcl_syntax::list::ListError::UnmatchedQuote.message(),
            ListError::BraceFollowedByJunk => {
                tcl_syntax::list::ListError::BraceFollowedByJunk.message()
            }
            ListError::QuoteFollowedByJunk => {
                tcl_syntax::list::ListError::QuoteFollowedByJunk.message()
            }
            ListError::NotUtf8 => "invalid list (not valid UTF-8)",
        }
        .as_bytes()
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

/// The byte-exact `Tcl_SplitList` error message for `src` (the `FindElement`
/// wording, `tclUtil.c`). For the `…followed by "X" instead of space` cases this
/// surfaces the offending fragment `X` — up to 20 non-space bytes after the
/// closing delimiter — which [`ListError::message`] alone cannot, since it has
/// no access to the position. Other variants render their fixed text.
///
/// Both halves come from the shared codec: the fragment walk from
/// [`tcl_syntax::list::junk_fragment`] and the sentence from
/// [`tcl_syntax::list::ListError::full_message`]. This function used to carry a
/// byte-identical re-implementation of that walk (issue #1429).
#[must_use]
pub fn list_error_message(src: &[u8], err: ListError) -> Vec<u8> {
    let shared = match err {
        ListError::UnmatchedBrace => tcl_syntax::list::ListError::UnmatchedBrace,
        ListError::UnmatchedQuote => tcl_syntax::list::ListError::UnmatchedQuote,
        ListError::BraceFollowedByJunk => tcl_syntax::list::ListError::BraceFollowedByJunk,
        ListError::QuoteFollowedByJunk => tcl_syntax::list::ListError::QuoteFollowedByJunk,
        // Not a shared variant: the runtime's own internal-rep invariant.
        ListError::NotUtf8 => return err.message().to_vec(),
    };
    // A non-UTF-8 `src` cannot reach the fragment walk; fall back to the fixed
    // text rather than lose the error entirely.
    match core::str::from_utf8(src) {
        Ok(s) => shared.full_message(s).into_bytes(),
        Err(_) => shared.message().as_bytes().to_vec(),
    }
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
    fn variable_scanner_consumes_colon_runs() {
        assert_eq!(scan_var_name(b"a:::b rest", 0), 5);
        assert_eq!(scan_var_name(b"::a:::b rest", 0), 7);
        assert_eq!(scan_var_name(b"foo::: rest", 0), 6);
        assert_eq!(scan_var_name(b"a:::b(k)", 0), 5);
    }

    #[test]
    fn array_index_source_mask_follows_the_release() {
        for (dialect, malformed) in [
            ("tcl8.4", false),
            ("tcl8.6", false),
            ("tcl9.0", true),
            ("tcl9.1", true),
        ] {
            let config = LexerConfig::for_dialect(dialect);
            let body = scan_parts(b"$a({key})", true, true, true, config);
            let parts = match body {
                WordBody::Parts(parts) => parts,
                WordBody::Literal(_) => panic!("variable must produce parts"),
            };
            assert_eq!(
                matches!(parts.as_slice(), [WordPart::ParseError(msg)] if *msg == tcl_lexer::INVALID_CHARACTER_IN_ARRAY_INDEX),
                malformed,
                "{dialect}: {parts:?}"
            );
        }

        let config = LexerConfig::for_dialect("tcl9.0");
        for source in [
            b"$a(\\{key\\})" as &[u8],
            b"$a(${key})",
            b"$a([format \\{])",
        ] {
            let body = scan_parts(source, true, true, true, config);
            assert!(
                !matches!(body, WordBody::Parts(ref parts) if parts.iter().any(|part| matches!(part, WordPart::ParseError(_)))),
                "Tcl 9 accepts escaped/substituted source: {source:?}"
            );
        }
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

    /// The word parser decodes literal runs under the *emulated release's*
    /// escape grammar, threaded on the `LexerConfig` the interpreter already
    /// builds from its dialect profile (issue #1479).
    #[test]
    fn word_escapes_decode_for_the_emulated_release() {
        // `\x4142`: all trailing hex digits keeping the low byte up to 8.5
        // (`B`), TIP 388's two-digit cap from 8.6 (`A42`). `\U` is 8.6+.
        for (dialect, hex, wide) in [
            ("tcl8.4", &b"B"[..], &b"U0001F600"[..]),
            ("tcl8.5", b"B", b"U0001F600"),
            ("tcl8.6", b"A42", "\u{FFFD}".as_bytes()),
            ("tcl9.0", b"A42", "\u{1F600}".as_bytes()),
            ("tcl9.1", b"A42", "\u{1F600}".as_bytes()),
        ] {
            let config = tcl_lexer::LexerConfig::for_dialect(dialect);
            let cmds = parse_script_with_config(b"puts \\x4142", config);
            let word = &cmds[0].words[1];
            assert_eq!(word_bytes(word), hex, "\\x4142 under {dialect}");
            let cmds = parse_script_with_config(b"puts \\U0001F600", config);
            let word = &cmds[0].words[1];
            assert_eq!(word_bytes(word), wide, "\\U0001F600 under {dialect}");
        }
    }

    /// A word's bytes whether it came back borrowed or decoded.
    fn word_bytes(w: &Word<'_>) -> Vec<u8> {
        match &w.body {
            WordBody::Literal(b) => (*b).to_vec(),
            WordBody::Parts(parts) => parts
                .iter()
                .flat_map(|p| match p {
                    WordPart::Text(t) => t.to_vec(),
                    other => panic!("expected only Text parts, got {other:?}"),
                })
                .collect(),
        }
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
    fn command_source_slice_matches_c() {
        // `src[start..end]` is the command string C logs in `::errorInfo`:
        // leading whitespace dropped, trailing whitespace kept, terminator
        // (`\n`/`;`) excluded. Verified byte-for-byte against tclsh 9.0.
        fn slice(src: &[u8]) -> (usize, Vec<u8>) {
            let cmds = parse_script(src);
            let c = &cmds[0];
            (c.start, src[c.start..c.end].to_vec())
        }
        // end-of-script: trailing space kept (the `[error deep ]` / `p ` cases).
        assert_eq!(slice(b" error deep "), (1, b"error deep ".to_vec()));
        // newline terminator excluded, no trailing space.
        assert_eq!(slice(b"error boom\n    "), (0, b"error boom".to_vec()));
        // trailing spaces before a newline terminator are kept.
        assert_eq!(slice(b"error boom   \n"), (0, b"error boom   ".to_vec()));
        // semicolon terminator excluded.
        assert_eq!(slice(b"a b ; c d"), (0, b"a b ".to_vec()));
        // a braced word: commandStart is at the `{`.
        assert_eq!(
            slice(b"  {} { error fromLambda }"),
            (2, b"{} { error fromLambda }".to_vec())
        );
    }

    #[test]
    fn command_line_numbers_are_body_relative() {
        // The 1-based line of a command = 1 + count('\n' in src[0..start]) —
        // the `(procedure "p" line N)` line. A proc body starts right after the
        // `{`, so this leading "\n" makes `error boom` land on line 3.
        let body = b"\n    set x 1\n    error boom\n";
        let cmds = parse_script(body);
        let line = |start: usize| 1 + body[..start].iter().filter(|&&b| b == b'\n').count();
        assert_eq!(line(cmds[0].start), 2); // `set x 1`
        assert_eq!(line(cmds[1].start), 3); // `error boom`
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

    // ---- component decomposition ----

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
                WordPart::Text(Cow::Borrowed(b"x")),
                WordPart::Variable(VarRef {
                    name: b"name",
                    index: None
                }),
                WordPart::Text(Cow::Borrowed(b"y")),
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

    /// Regression coverage for issue #996: `scan_parts` recurses once per
    /// `$name(index)` nesting level while parsing an array index's own
    /// substitution components, with no depth cap before this fix —
    /// reachable via ordinary `subst {...}`/variable substitution on nested
    /// array-index text, no special syntax needed. Empirically, this same
    /// class of unguarded nested-array-index recursion overflowed the native
    /// stack (SIGABRT) between depth 100-150 on a 256 KiB thread stack, still
    /// crashing at depth 2000 on a 1 MiB stack (this crate's own sweep; see
    /// `MAX_SCAN_PARTS_DEPTH`'s doc comment). 5000 is comfortably past both
    /// that crash range and `MAX_SCAN_PARTS_DEPTH` (64); the assertion is
    /// that parsing returns at all, not what it returns.
    #[test]
    fn deeply_nested_array_index_survives_scan_parts() {
        const DEPTH: usize = 5000;
        let mut src = String::from("$a0");
        for i in 0..DEPTH {
            src.push('(');
            src.push_str(&format!("$a{}", i + 1));
        }
        src.push('1');
        for _ in 0..DEPTH {
            src.push(')');
        }
        let _ = scan_parts(src.as_bytes(), true, true, true, LexerConfig::default());
    }

    /// A moderately nested array index (well under `MAX_SCAN_PARTS_DEPTH`)
    /// still scans into the full nested `Variable`/`index` component tree —
    /// the safety net must not fire, let alone flatten anything, on
    /// realistic nesting depths. (The trailing `Text("))")`s are a pre-
    /// existing, unrelated property of this scanner's `)`-terminator search
    /// — it does not skip over a nested `$name(…)`'s own parens the way
    /// `tcl_lexer`'s does, so only the *innermost* `)` closes each
    /// outer-to-inner index in a multi-level chain like this one; not a
    /// behaviour this fix changes.)
    #[test]
    fn moderately_nested_array_index_still_scans_fully() {
        // $a($b($c(1)))
        let body = scan_parts(b"$a($b($c(1)))", true, true, true, LexerConfig::default());
        assert_eq!(
            body,
            WordBody::Parts(vec![
                WordPart::Variable(VarRef {
                    name: b"a",
                    index: Some(vec![WordPart::Variable(VarRef {
                        name: b"b",
                        index: Some(vec![WordPart::Variable(VarRef {
                            name: b"c",
                            index: Some(vec![WordPart::Text(Cow::Borrowed(&b"1"[..]))]),
                        })]),
                    })]),
                }),
                WordPart::Text(Cow::Borrowed(&b"))"[..])),
            ])
        );
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
