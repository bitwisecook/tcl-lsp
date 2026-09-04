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
//! ## Where the pieces live
//!
//! Nothing here scans source any more. The **within-word** decomposition is
//! [`tcl_lexer::word_parts`]' — [`WordPart`], [`VarRef`], [`WordBody`] and the
//! scan that produces them are re-exports of it, the one owner shared with
//! `tcl-vm` and the compiler's segmenter. The **command and word boundaries**
//! are [`tcl_lexer::script::group_commands`]' (issue #1786), the same grouping
//! the compiler's CST builder consumes. What is left in this module is the
//! *lowering*: turning the owner's borrow-free spans into this crate's
//! borrow-based `Command`/`Word` tree, and applying the eval-facing
//! word-delimiter rules on the way (`{braced}` is literal; `"quoted"` must
//! close; text welded onto a close-brace is C's `extra characters after
//! close-brace`).
//!
//! This crate used to carry its own copy of the decomposer (`scan_parts`,
//! `scan_var_name`, `skip_command_subst`, `parse_var_ref`) with `subst.rs`
//! mirroring it — two of the four copies bucket R10 found — and its own copy
//! of the boundary loop, which disagreed with the compiler's segmenter about
//! `{*}` after a close-brace (`{a}{*}$b`: one welded `Bare` word here, two
//! words there, and an error in C). They are gone.

#![forbid(unsafe_code)]

use std::borrow::Cow;

use tcl_lexer::script::{group_commands, WordSpan};
use tcl_lexer::{Lexer, LexerConfig, SourceMap, Token, TokenType};

/// How a word was delimited in the source: `Bare` (`foo`, `$x`, `a[b]c`),
/// `Quoted` (`"a $b"` — substitutions active, quotes stripped), or `Braced`
/// (`{a $b}` — pure literal, braces stripped, no substitution).
///
/// This crate's own copy of the enum is gone: the rule that decides the kind
/// moved to the boundary owner along with the grouping loop, and this is a
/// re-export of [`tcl_lexer::script::WordKind`] (which was lifted from here,
/// so the variants and their meaning are unchanged).
pub use tcl_lexer::script::WordKind;

// The word-component model is the shared owner's — re-exported under the
// names this crate's evaluator (`interp.rs`) and `subst` already use, so the
// consumers stay put while the implementation moved down a crate.
pub use tcl_lexer::word_parts::{VarRef, WordBody, WordPart};

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

/// Decompose a span into substitution components, with each substitution kind
/// independently enabled (`do_vars`/`do_cmds`/`do_bs` ↔ `subst`'s
/// `-novariables`/`-nocommands`/`-nobackslashes`).
///
/// A thin adapter over the shared owner
/// [`tcl_lexer::word_parts::decompose`] — kept as this crate's spelling
/// because `subst` and the word builder both call it with three booleans, not
/// a struct. All the semantics (C's exact parse-error messages, the
/// release-variant `${…}` and array-index rules, the borrow fast path) are the
/// owner's.
pub fn scan_parts(
    src: &[u8],
    do_vars: bool,
    do_cmds: bool,
    do_bs: bool,
    config: LexerConfig,
) -> WordBody<'_> {
    tcl_lexer::word_parts::decompose(
        src,
        tcl_lexer::word_parts::SubstFlags {
            vars: do_vars,
            cmds: do_cmds,
            backslashes: do_bs,
            bare_var_refs: true,
            // Source, so an unclosed `[` keeps C's parse error.
            ..tcl_lexer::word_parts::SubstFlags::default()
        },
        config,
    )
}

// ---------------------------------------------------------------------------
// Command parser.
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Command/word parsing — LOWERED from the canonical `tcl-lexer` token stream
// and the canonical `tcl_lexer::script` grouping of it (the "parse once"
// convergence: one scanner and one boundary rule shared with the
// LSP/compiler). The hard edges (`{*}` prefix, `#`-comment-in-command-position,
// brace/quote/bracket nesting, `$arr(idx)`, line continuation) live in
// `tcl-lexer`; here we only map its commands/words into the eval
// `Command`/`WordPart` model. `scan_parts` remains for `subst`; `split_list`
// delegates to the shared `tcl_syntax::list` crate.
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
    // The boundary question — where each command and each word begins and
    // ends — is the owner's (issue #1786); this crate used to answer it with
    // its own copy of the loop, which disagreed with the compiler's segmenter
    // on `{*}` welded to a close-brace. `group_commands` is borrow-free
    // (spans + token indices), so lowering its answer into `Command`/`Word`
    // keeps this crate's zero-copy contract (memory-management.md MM-B.6):
    // the words below still borrow `src`.
    group_commands(&toks, s, config)
        .into_iter()
        .map(|cmd| {
            // C's `commandStart`: the first content token, leading whitespace
            // and any preceding comment already skipped.
            let start = cmd.span.start() as usize;
            // The terminator (`\n`/`;`) — its span *start* is the first byte
            // past the command's content, so `src[start..end]` keeps trailing
            // whitespace but drops the terminator, and its span *end* is where
            // the next command resumes. The lexer always closes a stream with
            // a zero-width `Eol`, so `None` only reaches here from a
            // hand-built token slice; end of source is then both answers.
            let (end, next) = cmd.terminator.map_or((src.len(), src.len()), |i| {
                (toks[i].span.start() as usize, toks[i].span.end() as usize)
            });
            Command {
                words: cmd
                    .words
                    .iter()
                    .map(|w| build_word(&sm, src, &toks[w.tokens.clone()], w, config))
                    .collect(),
                next,
                start,
                end,
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// C's script-parsing order: a command parses whole, THEN evaluates.
// ---------------------------------------------------------------------------

/// How deep a command's `[…]` nesting is pre-scanned before the scan gives up
/// and lets the error surface the old way (during substitution).
///
/// Each `[…]` level of a command is one `eval_command_subst` — i.e. one
/// `Interp::eval_depth` step — when the command actually runs, so a script
/// nested deeper than the interpreter's own `NATIVE_EVAL_DEPTH_LIMIT` could
/// never have evaluated anyway. Capping at the same budget keeps the scan's
/// native recursion bounded by a constant instead of by the input.
const MAX_PARSE_ERROR_SCAN_DEPTH: u32 = 128;

/// The first parse error anywhere in `words`, in source order — `None` if the
/// whole command parses.
///
/// C's `Tcl_EvalEx` calls `Tcl_ParseCommand` for **one whole command** and only
/// then substitutes and dispatches its words, so a parse failure in *any* word
/// is the command's failure and nothing the command would have substituted
/// runs. Measured on tclsh 8.6.16 and 9.0.4, both identical:
///
/// ```text
/// proc sfx {t} { lappend ::ran $t; return S$t }
/// list [sfx inner] {a}b            -> extra characters after close-brace, ::ran empty
/// list [sfx inner] "unterminated   -> missing "                          , ::ran empty
/// list [sfx inner] [sfx two        -> missing close-bracket              , ::ran empty
/// puts "[sfx inner]pre${abc"       -> missing close-brace for variable name, ::ran empty
/// ```
///
/// The scan descends into `[…]` bodies and `$arr(index)` components because
/// C's parse does: `Tcl_ParseCommand`'s `ParseTokens` recurses into a bracket,
/// parsing the substituted script's own commands, so an error *inside* a later
/// bracket also stops an earlier bracket from running —
/// `list [sfx inner] [list "oops]` raises `missing "` with `::ran` empty on
/// both shells. A `{braced}` word is a [`WordBody::Literal`] and holds no
/// components, so it is not descended into — matching C, which does not parse
/// a braced word's contents as a script either.
///
/// Note the deliberate non-consumer: `subst` does **not** get this treatment.
/// It is not a command parse — C substitutes its template left to right and
/// keeps the side effects of every `[…]` that ran before the failure
/// (`subst {[side][b}` prints `side`'s output, then raises), which is exactly
/// what walking [`WordPart`]s in order already does.
#[must_use]
pub fn first_parse_error(words: &[Word<'_>], config: LexerConfig) -> Option<&'static str> {
    SCAN_MEMO.with(|memo| {
        let memo = &mut *memo.borrow_mut();
        memo.truncated = false;
        words_parse_error(words, config, 0, memo)
    })
}

/// The `[…]` recursion's memo, and how big it is allowed to get before it is
/// dropped wholesale.
///
/// The scan's answer is a pure function of (script bytes, [`LexerConfig`]), so
/// memoizing it is sound — and it has to be memoized, because this crate has no
/// parse cache by design (the borrow-based tree makes one a lifetime hazard,
/// memory-management.md MM-B.6). Without it, every execution of a command
/// re-parses each of its `[…]` bodies once for the scan on top of the parse the
/// substitution itself does, which on a bracket-dense loop measured ~40% slower.
///
/// The memo owns its keys and stores a `&'static str`, so nothing here can
/// dangle; it is per-thread rather than per-[`crate::interp::Interp`] precisely
/// because the function is pure — two interpreters on one thread asking the same
/// question deserve the same answer. The cap bounds the memory an adversarial
/// script (a fresh `[…]` body per iteration) can pin.
const SCAN_MEMO_CAP: usize = 4096;

thread_local! {
    static SCAN_MEMO: std::cell::RefCell<ScanMemo> = std::cell::RefCell::new(ScanMemo::default());
}

#[derive(Default)]
struct ScanMemo {
    /// The config every entry was computed under; a different one clears it
    /// (a version-pinned interpreter changes the grammar, issue #1462).
    config: Option<LexerConfig>,
    entries: std::collections::HashMap<Vec<u8>, Option<&'static str>>,
    /// Set while a subtree's scan was cut short by
    /// [`MAX_PARSE_ERROR_SCAN_DEPTH`]. A truncated answer is only valid at the
    /// depth it was computed at, so it must not be memoized — the same script
    /// reached at a shallower depth would scan further and could find an error
    /// this run did not.
    truncated: bool,
}

impl ScanMemo {
    fn get(&mut self, script: &[u8], config: LexerConfig) -> Option<Option<&'static str>> {
        if self.config != Some(config) {
            self.config = Some(config);
            self.entries.clear();
            return None;
        }
        self.entries.get(script).copied()
    }

    fn put(&mut self, script: &[u8], answer: Option<&'static str>) {
        if self.entries.len() >= SCAN_MEMO_CAP {
            self.entries.clear();
        }
        self.entries.insert(script.to_vec(), answer);
    }
}

fn words_parse_error(
    words: &[Word<'_>],
    config: LexerConfig,
    depth: u32,
    memo: &mut ScanMemo,
) -> Option<&'static str> {
    words.iter().find_map(|w| match &w.body {
        WordBody::Literal(_) => None,
        WordBody::Parts(parts) => parts_parse_error(parts, config, depth, memo),
    })
}

fn parts_parse_error(
    parts: &[WordPart<'_>],
    config: LexerConfig,
    depth: u32,
    memo: &mut ScanMemo,
) -> Option<&'static str> {
    if depth >= MAX_PARSE_ERROR_SCAN_DEPTH {
        memo.truncated = true;
        return None;
    }
    parts.iter().find_map(|part| match part {
        WordPart::ParseError(msg) => Some(*msg),
        WordPart::Text(_) => None,
        WordPart::Variable(v) => v
            .index
            .as_deref()
            .and_then(|idx| parts_parse_error(idx, config, depth + 1, memo)),
        WordPart::Command(script) => script_parse_error(script, config, depth + 1, memo),
    })
}

fn script_parse_error(
    script: &[u8],
    config: LexerConfig,
    depth: u32,
    memo: &mut ScanMemo,
) -> Option<&'static str> {
    if let Some(hit) = memo.get(script, config) {
        return hit;
    }
    let enclosing_truncated = std::mem::replace(&mut memo.truncated, false);
    let answer = parse_script_with_config(script, config)
        .iter()
        .find_map(|cmd| words_parse_error(&cmd.words, config, depth, memo));
    if !memo.truncated {
        memo.put(script, answer);
    }
    memo.truncated |= enclosing_truncated;
    answer
}

/// C's parse errors for the word delimiters this module owns — spelled by
/// the shared owner, not re-typed here.
///
/// The lexer itself stays lenient about all three — it is shared with the LSP,
/// which must keep tokenizing broken source — so this eval-facing parser is
/// the one that fails closed, carrying the failure as a
/// [`WordPart::ParseError`] the evaluator raises when it reaches the word
/// (issues #1576, #1586).
use tcl_lexer::word_parts::{EXTRA_AFTER_CLOSE_BRACE, MISSING_CLOSE_BRACE, MISSING_QUOTE};

/// Whether a `Str` token that **opens a brace** never found its closing `}`
/// before end of input. Delegates to [`tcl_lexer::word_closer_offset`] — the
/// one owner of "does this delimited word's span actually reach its closer" —
/// rather than re-deriving the answer from span arithmetic here.
///
/// The opening-byte test is load-bearing, not defensive. `Str` is the lexer's
/// *literal-fragment* class, not its brace class: a `$` that starts no
/// variable reference is one too. Without the test, `word_closer_offset`
/// answers "no closer" for that `$` and every one of these — all accepted by
/// 8.4.20, 8.5.19, 8.6.16, 9.0.4 and 9.1b0 — raised `missing close-brace`:
///
/// ```text
/// puts "$"           -> $
/// puts "50% of $"    -> 50% of $
/// puts "a$ b"        -> a$ b
/// puts a$%b          -> a$%b
/// ```
///
/// C reads a `$` that no name follows as the text `$` and keeps parsing
/// (`Tcl_ParseVarName` form 3, `justADollarSign`,
/// tmp/tcl9.0.4/generic/tclParse.c:1454). Found by
/// `tests/parse_cut_agreement.rs` against `tcl_lexer::first_parse_cut`, on
/// tcllib's `markdown.test`.
fn brace_token_unterminated(sm: &SourceMap<'_>, tok: Token) -> bool {
    sm.source().as_bytes().get(tok.span.start() as usize) == Some(&b'{')
        && tcl_lexer::word_closer_offset(sm, tok).is_none()
}

fn build_word<'s>(
    sm: &SourceMap<'s>,
    src: &'s [u8],
    toks: &[Token],
    word: &WordSpan,
    config: LexerConfig,
) -> Word<'s> {
    let escapes = config.escapes;
    let expand = word.expand;
    let kind = word.kind;
    let start = toks[0].span.start() as usize;
    // Text welded straight onto a close-brace (`{a}b`, `{a}$b`, `{a}[b]`,
    // `{}x`, `{a}{b}`, `{a}{*}$b`) is C's `extra characters after
    // close-brace`, raised while the *command* is parsed: measured on 8.6.16
    // and 9.0.4, `list [side] {a}b` reports it without running `side`. Both
    // Rust groupers used to accept the shape instead — and disagreed on what
    // it meant (this crate welded `{a}` and `$b` into one `Bare` word; the
    // compiler's segmenter split them) — so the boundary owner records the
    // weld in `WordSpan::welded_after_close` and the eval-facing parser is
    // the one that fails closed on it. Checked before anything else in the
    // word, because C stops at the close-brace: the fragments after it are
    // never parsed, so their own errors (and side effects) never surface.
    if word.welded_after_close {
        return Word {
            kind,
            expand,
            body: WordBody::Parts(vec![WordPart::ParseError(EXTRA_AFTER_CLOSE_BRACE)]),
            start,
        };
    }
    // A single braced token is a literal word (no substitution) — except a
    // backslash-newline line continuation, the one backslash sequence a braced
    // word collapses (to a single space, swallowing following spaces/tabs), as
    // C does in its pre-parse pass. A word with no continuation borrows the
    // source unchanged; one with a continuation owns the collapsed bytes.
    if kind == WordKind::Braced {
        if brace_token_unterminated(sm, toks[0]) {
            // C aborts parsing outright here (`missing close-brace`); this
            // scanner stays infallible by carrying the failure as a
            // `ParseError` part for the evaluator to raise when it reaches
            // this word — the same convention `${…}` uses (issue #1586).
            return Word {
                kind,
                expand,
                body: WordBody::Parts(vec![WordPart::ParseError(MISSING_CLOSE_BRACE)]),
                start,
            };
        }
        let content = token_content(sm, src, toks[0]);
        let body = match tcl_syntax::backslash::collapse_brace_continuations(content) {
            Cow::Borrowed(b) => WordBody::Literal(b),
            Cow::Owned(o) => WordBody::Parts(vec![WordPart::Text(Cow::Owned(o))]),
        };
        return Word {
            kind,
            expand,
            body,
            start,
        };
    }
    // A `"`-delimited word that never closes is C's `missing "`
    // (`Tcl_ParseQuotedString`), and it aborts the parse — `eval {list a "b}`
    // raises on 8.6.16 and 9.0.4 rather than reading `b` as the word. The
    // lexer's recovery reads to end of input instead, so the close is
    // re-derived from the shared owner, exactly as the braced word above does.
    //
    // It is a *fallback*, not a verdict: `quoted_word_close` steps over
    // complete `[…]` substitutions to find the closer, so an **incomplete**
    // one makes it give up here while C, parsing the word's tokens left to
    // right, has already failed inside the bracket. Measured on 8.6.16 and
    // 9.0.4, `puts "[foo"` is `missing close-bracket`, not `missing "`. So
    // the word's parts are still built below and this error is appended
    // after them, where the first-error rule reaches it only if nothing
    // inside the word failed first.
    let unterminated_quote = kind == WordKind::Quoted
        && core::str::from_utf8(src)
            .is_ok_and(|text| tcl_lexer::word_parts::quoted_word_close(text, start).is_err());
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
            // A braced fragment mid-word (rare, e.g. the glued `{a}{b` second
            // half) is verbatim apart from the backslash-newline line
            // continuation, which collapses to a space — unless it never
            // closes, in which case it's necessarily the last token in the
            // word (nothing follows end of input) and fails the same way the
            // single-token fast path above does.
            TokenType::Str => {
                if brace_token_unterminated(sm, t) {
                    parts.push(WordPart::ParseError(MISSING_CLOSE_BRACE));
                } else {
                    parts.push(WordPart::Text(
                        tcl_syntax::backslash::collapse_brace_continuations(bytes),
                    ));
                }
            }
            // Both `$` spellings resolve through the shared owner, reading
            // the *source* at the token's `$` rather than the token's own
            // content. The lexer's boundary is the lenient recovery it owes
            // the LSP (an unterminated `${` reads a name running to end of
            // input; an unterminated `$a(` reads `a(` as a name), so trusting
            // it here would let direct Rust/WASM evaluation bypass the errors
            // C raises: `missing close-brace for variable name` (#1586),
            // `invalid character in array index` (#1732), `missing )`.
            TokenType::Var => {
                match tcl_lexer::word_parts::scan_var_ref(src, t.span.start() as usize, config) {
                    Ok(Some(raw)) => {
                        let index =
                            raw.index
                                .map(|idx| match scan_parts(idx, true, true, true, config) {
                                    WordBody::Literal(b) => vec![WordPart::Text(Cow::Borrowed(b))],
                                    WordBody::Parts(p) => p,
                                });
                        parts.push(WordPart::Variable(VarRef {
                            name: raw.name,
                            index,
                        }));
                    }
                    // A `Var` token whose `$` opens no reference cannot come
                    // out of the lexer; keep the bytes as text rather than
                    // dropping the word's content if one ever did.
                    Ok(None) => parts.push(WordPart::Text(Cow::Borrowed(bytes))),
                    Err(msg) => parts.push(WordPart::ParseError(msg)),
                }
            }
            // A `[…]` that never closes is C's `missing close-bracket` — or
            // whatever error C meets first *inside* the bracket, since it
            // parses the substituted script rather than hunting for the `]`.
            // The lexer's recovery reads to end of input and hands back a
            // `Cmd` token regardless, so the close is re-derived from the
            // shared owner, exactly as the `{` and `"` words above are.
            TokenType::Cmd => {
                match tcl_lexer::word_parts::command_subst_close(
                    src,
                    t.span.start() as usize,
                    tcl_lexer::word_parts::SubstFlags::default(),
                    config,
                ) {
                    Ok(_) => parts.push(WordPart::Command(bytes)),
                    Err(msg) => parts.push(WordPart::ParseError(msg)),
                }
            }
            _ => {}
        }
    }
    if unterminated_quote {
        parts.push(WordPart::ParseError(MISSING_QUOTE));
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
            start,
        };
    }
    if let [WordPart::Text(Cow::Borrowed(b))] = parts.as_slice() {
        return Word {
            kind,
            expand,
            body: WordBody::Literal(b),
            start,
        };
    }
    Word {
        kind,
        expand,
        body: WordBody::Parts(parts),
        start,
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

    /// The `n`th command of `src`. Replaces the old `parse_command(src, pos)`
    /// resume-by-offset entry point, which had no non-test caller once the
    /// boundary loop moved to `tcl_lexer::script` and was deleted with it.
    fn nth(src: &[u8], n: usize) -> Command<'_> {
        parse_script(src).into_iter().nth(n).expect("command")
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

    // ---- command / word boundaries ----

    #[test]
    fn two_word_command() {
        let c = nth(b"puts hello", 0);
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
        let c = nth(b"set x {hello world}", 0);
        assert_eq!(c.words.len(), 3);
        assert_eq!(c.words[2].kind, WordKind::Braced);
        assert_eq!(lit(&c.words[2]), b"hello world");
    }

    #[test]
    fn quoted_word_strips_quotes() {
        let c = nth(b"puts \"hi there\"", 0);
        assert_eq!(c.words.len(), 2);
        assert_eq!(c.words[1].kind, WordKind::Quoted);
        assert_eq!(lit(&c.words[1]), b"hi there");
    }

    #[test]
    fn semicolon_ends_command() {
        let c = nth(b"a b ; c d", 0);
        assert_eq!(c.words.len(), 2);
        assert_eq!(lit(&c.words[0]), b"a");
        assert_eq!(lit(&c.words[1]), b"b");
        // the `;` terminator ends the command and the next one resumes past
        // it: `end` is the terminator's span *start* (so `src[start..end]` is
        // `"a b "`, trailing space kept), `next` its span *end* (past the
        // `"; "` the lexer folds into one `Eol`).
        assert_eq!(&b"a b ; c d"[c.start..c.end], b"a b ");
        assert_eq!(c.next, 6);
        let c2 = nth(b"a b ; c d", 1);
        assert_eq!(c2.start, c.next);
        assert_eq!(lit(&c2.words[0]), b"c");
        assert_eq!(lit(&c2.words[1]), b"d");
    }

    #[test]
    fn comment_is_not_a_command() {
        assert!(parse_script(b"# this is a comment\n").is_empty());
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
        let c = nth(b"foo {*}$args bar", 0);
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
        let c = nth(b"foo {*}", 0);
        assert_eq!(c.words.len(), 2);
        assert!(!c.words[1].expand);
        assert_eq!(c.words[1].kind, WordKind::Braced);
        assert_eq!(lit(&c.words[1]), b"*");
        // `{*} x` (space after) → literal `{*}` then `x`, NOT expansion
        let c = nth(b"foo {*} x", 0);
        assert_eq!(c.words.len(), 3);
        assert!(!c.words[1].expand);
        assert_eq!(lit(&c.words[1]), b"*");
        assert!(!c.words[2].expand);
        assert_eq!(lit(&c.words[2]), b"x");
        // `{*}{a b}` → expansion of the braced word `a b`
        let c = nth(b"foo {*}{a b}", 0);
        assert_eq!(c.words.len(), 2);
        assert!(c.words[1].expand);
        assert_eq!(lit(&c.words[1]), b"a b");
    }

    #[test]
    fn line_continuation_joins_logical_line() {
        let c = nth(b"cmd a \\\n   b c", 0);
        let got: Vec<&[u8]> = c.words.iter().map(lit).collect();
        assert_eq!(got, vec![&b"cmd"[..], b"a", b"b", b"c"]);
    }

    // ---- component decomposition ----

    #[test]
    fn bracket_subst_decomposes_in_one_word() {
        let c = nth(b"set x [clock seconds]", 0);
        assert_eq!(c.words.len(), 3);
        assert_eq!(parts(&c.words[2]), &[WordPart::Command(b"clock seconds")]);
    }

    #[test]
    fn mixed_text_and_variable() {
        let c = nth(b"puts x${name}y", 0);
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
        let c = nth(b"puts $arr($i)", 0);
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
    /// realistic nesting depths. The scanner's `)`-terminator search now
    /// skips a nested `$name(…)`'s own parens, so each level closes on its
    /// matching `)` and nothing spills out as trailing text — the C Tcl
    /// reading (`set c(1) inner; set b(inner) mid; set a(mid) outer;
    /// set x $a($b($c(1)))` yields `outer` under `tclsh9.0`).
    #[test]
    fn moderately_nested_array_index_still_scans_fully() {
        // $a($b($c(1)))
        let body = scan_parts(b"$a($b($c(1)))", true, true, true, LexerConfig::default());
        assert_eq!(
            body,
            WordBody::Parts(vec![WordPart::Variable(VarRef {
                name: b"a",
                index: Some(vec![WordPart::Variable(VarRef {
                    name: b"b",
                    index: Some(vec![WordPart::Variable(VarRef {
                        name: b"c",
                        index: Some(vec![WordPart::Text(Cow::Borrowed(&b"1"[..]))]),
                    })]),
                })]),
            }),])
        );
    }

    #[test]
    fn literal_word_is_zero_copy() {
        // A word with no $ [ \ is a borrow of the source, not a Vec.
        let src = b"plainword rest";
        let c = nth(src, 0);
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
