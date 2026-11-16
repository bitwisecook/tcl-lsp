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

//! **Structural-state recovery index (script bracket dimension).**
//!
//! This module implements the structural-state index designed in
//! `docs/design/compiler/error-recovery.md`: capture the index in one
//! scan, then answer *"does inserting a closer at offset O balance the
//! outermost open delimiter?"* from the index alone — **no re-lex** —
//! with the answer proven to match a real re-scan.
//!
//! Scope: all three structural-index dimensions the doc names — the
//! script **`[`/`]` (bracket)** and **`{`/`}` (brace)** sublanguages
//! ([`BracketIndex`], [`BraceIndex`]) and the expr **`(`/`)` (paren)**
//! sublanguage ([`ExprParenIndex`]).
//!
//! Two of those dimensions are live in production. [`BracketIndex`] is
//! built by the analyser's E201 path
//! (`tcl-compiler/src/analyser/syntax_checks.rs::detect_e201`), whose
//! [`BracketIndex::is_inert`] check vetoes any proposed `]` insertion
//! that would land in a literal span; [`BracketIndex`] and
//! [`BraceIndex`] also back the compiler explorer's `structuralIndex`
//! view (`tcl-explorer/src/serialise.rs`). [`ExprParenIndex`] is
//! currently exercised only by this crate's tests. The incremental
//! green-tree engine will build on the same index.
//!
//! The **expr paren** dimension is built directly from the expr lexer's
//! token stream (`$arr(idx)` is one `Variable`; strings / command subs
//! are whole tokens), so its grouping-paren count is the lexer's — the
//! cleanest realisation of "store the lexer's entry-state per token". It
//! is verified against C Tcl 9.0.3 `expr`'s `unbalanced open/close paren`
//! diagnostics over an adversarial fuzz corpus.
//!
//! ### Brace-dimension command-sub interiors (recursive script parse)
//!
//! `info complete` parses a `[…]` **interior recursively as a script**
//! (word-based braces + terminal "extra characters after close-brace"),
//! so `[set x {b}{` is **complete** even though the outer `[` is
//! unterminated. The brace scanner mirrors this: [`BraceBuilder`]'s
//! `scan_script` handles both the top level and a command-sub interior
//! (terminating at the matching `]`), and a terminal extra-chars error
//! propagates up through every enclosing scope. Verified against C Tcl
//! 9.0.3 across the full adversarial corpus (command subs included), not
//! just the bracketless / bracket-isolated subsets.
//!
//! ## The two contexts the scanner must mirror (the doc's warning)
//!
//! A scalar bracket level is **not** enough. Whether a `[` / `]` is
//! *structural* or *inert* depends on the surrounding sublanguage, and
//! Tcl has **two different brace rules** that the index must reproduce
//! exactly:
//!
//! * **Top level / quoted words** — `{` opens a *brace word* only at a
//!   word boundary (`is_newword`); the whole verbatim `{…}` span is
//!   inert for brackets. `"` toggles a quoted run (brackets still count
//!   inside it; braces are literal).
//! * **Inside a `[…]` command substitution** — brace handling is
//!   *count-based*, not word-based (mirrors
//!   `Lexer::scan_command_substitution`): `{` / `}` adjust `blevel`, and
//!   a `]` only closes when `blevel == 0 && !in_quotes`. So a `]` inside
//!   `{…}` inside `[…]` is inert.
//!
//! Plus the inert leaves shared by both: backslash-escape pairs and
//! `${…}` variable-name braces.
//!
//! ## Two corrections the scanner must get right
//!
//! 1. **An extra closer closes nothing** — the running level clamps at
//!    0 (a `]` with nothing open is literal, not a negative level).
//! 2. **An unterminated opaque token reaching EOF makes the tail inert**
//!    — an unterminated `[…` / `"…` / `${…` swallows to EOF; here that
//!    falls out naturally (the open `[` stays open and a later inserted
//!    `]` closes *it*).
//!
//! ## Validation (this module's `#[cfg(test)]` harness)
//!
//! The harness fuzzes thousands of deterministic snippets and verifies:
//!
//! * **Faithfulness to the production lexer** — the index's
//!   unterminated-`[` verdict matches `Lexer`'s `missing close-bracket`
//!   warning on **8000/8000** fuzz cases.
//! * **C Tcl 9.0.3 reference iff** — on a corpus that isolates the
//!   bracket dimension (balanced, word-separated braces/quotes), the
//!   index's verdict matches `tclsh9.0`'s `info complete` **both ways**
//!   (the reference standard; skipped only if `tclsh9.0` is absent).
//! * **Realistic recovery vs C Tcl 9.0.3** — a single forgotten `]` in
//!   real code: the index predicts the close site and the repair is
//!   reference-complete.
//! * **The two corrections** (extra-closer-clamp, opaque-to-EOF) and the
//!   **scalar-index-diverges** demonstration.
//!
//! ### Documented divergences
//!
//! 1. **`info complete` ≠ tokenisation on "extra characters after a
//!    close-brace/quote".** C Tcl 9.0.3 reports `{b}[` as *complete* (the
//!    `[` is a terminal "extra characters" error, not a command
//!    substitution), whereas the lexer/index see an unterminated `[`.
//!    A benign over-report for recovery; pinned by
//!    `ctcl9_extra_chars_after_word_is_a_documented_divergence`.
//! 2. **The naive "replay the prebuilt index with one inserted closer"
//!    is NOT sound for adversarial multi-bracket input** (~88% self-
//!    consistency): inserting a `]` can close a bracket early and
//!    re-contextualise the tail (count-based command-sub interior ⇄
//!    top-level word-based), split a `\\` pair, or be swallowed by a
//!    following quote. It *is* exact for the realistic single-`]` case.
//!    The productionised forward-walk must re-derive tail context after a
//!    hypothetical close — the doc's "command-sub interiors need care".

use tcl_core_types::RecursionLimit;

use crate::substitution::{backslash_continuation_end, backslash_escape_end, is_line_continuation};

/// Cap on nested `[…]` command-substitution depth for every recursive
/// scanner in this module ([`BracketIndex`]'s `scan_cmd_sub`,
/// [`BraceIndex`]'s `scan_script`/`scan_quoted`, and the
/// [`script_is_complete`]/[`command_boundaries`] scanner's
/// `scan_complete`/`scan_complete_quoted`). Each recurses one native-stack
/// frame group per `[`, with no natural bound — `[a [a [a …]]]` nested N
/// deep recurses N deep — so pathologically deep input (generated or
/// minified Tcl) could otherwise abort the process with
/// an uncatchable stack overflow. 128 is far past any bracket nesting
/// real Tcl code uses; past it, a further `[` is scanned as an ordinary
/// character instead of recursed into, so the index/completeness verdict
/// degrades gracefully rather than crashing. See
/// `docs/design/compiler/recursive-descent-depth-limits.md`.
const MAX_NESTED_BRACKET_DEPTH: RecursionLimit = RecursionLimit(128);

/// A structural-state index for the script bracket (`[` / `]`)
/// dimension, captured in a single forward scan.
#[derive(Debug, Clone)]
pub struct BracketIndex {
    /// Structural bracket events in ascending offset order: `+1` for a
    /// structural `[`, `-1` for a structural `]`. Brackets inside inert
    /// spans are *not* recorded.
    events: Vec<(u32, i32)>,
    /// Inert byte ranges where `[` / `]` are literal (brace words,
    /// `${…}`, escape pairs, command-sub `{…}` interiors).  Sorted,
    /// non-overlapping.  `(start, end, terminated)`: `terminated` is
    /// `true` when the span is closed by its own delimiter (a `}` for
    /// brace words, `}` for `${…}`, `]` for cmd-sub brakes).  An
    /// unterminated span is `(start, EOF, false)` — its end equals
    /// `self.len` and was never closed, so it conceptually swallows
    /// past EOF and insertion at the end is still absorbed.
    inert: Vec<(u32, u32, bool)>,
    /// Source length in bytes.
    len: u32,
}

impl BracketIndex {
    /// Build the index for `source` (one forward scan).
    #[must_use]
    pub fn build(source: &str) -> Self {
        Self::build_with_config(source, crate::LexerConfig::default())
    }

    /// Build source structural geometry with the actual retained lexical axes.
    /// The braced-variable inertness follows this configuration. Existing
    /// structural recovery approximations remain independent of tokenisation,
    /// command binding and semantic execution.
    #[must_use]
    pub fn build_with_config(source: &str, config: crate::LexerConfig) -> Self {
        let bytes = source.as_bytes();
        let mut b = Builder {
            source,
            bytes,
            config,
            events: Vec::new(),
            inert: Vec::new(),
        };
        b.scan_top();
        // Inert spans are emitted in scan order, but a brace run that
        // runs to EOF is pushed *after* the escape pairs nested inside
        // it, so the raw list is not sorted. Sort by start and merge
        // overlaps/adjacencies so `is_inert`'s binary search is valid.
        b.inert.sort_unstable();
        let mut merged: Vec<(u32, u32, bool)> = Vec::with_capacity(b.inert.len());
        for (s, e, term) in b.inert {
            match merged.last_mut() {
                // Merge overlapping/adjacent spans.  An unterminated
                // span always reaches EOF; merging into it preserves
                // its unterminated flag unless the merged span is
                // terminated beyond EOF (impossible — EOF is max).
                Some(last) if s <= last.1 => {
                    last.1 = last.1.max(e);
                    last.2 = last.2 || term;
                }
                _ => merged.push((s, e, term)),
            }
        }
        BracketIndex {
            events: b.events,
            inert: merged,
            len: u32::try_from(bytes.len()).expect("source length fits u32"),
        }
    }

    /// `true` when `off` falls inside an inert span — `start <= off <
    /// end`. Used for membership queries (e.g. "is this byte literal").
    #[must_use]
    pub fn is_inert(&self, off: u32) -> bool {
        let idx = self.inert.partition_point(|&(s, _, _)| s <= off);
        if idx == 0 {
            return false;
        }
        let (_, end, _) = self.inert[idx - 1];
        off < end
    }

    /// `true` when inserting a closer *at* `off` would be absorbed as a
    /// literal — i.e. `off` is **strictly inside** an inert span (`start
    /// < off < end`). Insertion at a span's start sits *before* the
    /// inert run (e.g. before a `\` or a `{`), so the closer is
    /// structural there; only insertion between the bytes is absorbed
    /// (e.g. `\` + inserted `]` -> `\]`).
    ///
    /// When the inert span is **unterminated** (its end equals
    /// `self.len` and it was never closed by its own delimiter),
    /// insertion at EOF is also absorbed — the unterminated span
    /// conceptually swallows past the source end, so a byte inserted
    /// there becomes part of the span's content.
    #[must_use]
    fn inert_for_insert(&self, off: u32) -> bool {
        let idx = self.inert.partition_point(|&(s, _, _)| s < off);
        if idx == 0 {
            return false;
        }
        let (start, end, terminated) = self.inert[idx - 1];
        if !terminated && end == self.len {
            // An unterminated inert span swallows to EOF, so an
            // insertion anywhere past its start (including at EOF)
            // is absorbed — the nearer-to-EOF span (e.g.
            // unterminated brace word) owns every following byte.
            start < off && off <= end
        } else {
            // Terminated spans: insertion is absorbed only at
            // strictly interior bytes (start < off < end).
            start < off && off < end
        }
    }

    /// Number of unterminated `[` at EOF — the final clamped bracket
    /// level. `0` means every `[` is matched (extra `]` are literal and
    /// clamp away).
    #[must_use]
    pub fn unterminated_count(&self) -> i32 {
        let mut lvl = 0i32;
        for &(_, d) in &self.events {
            lvl = (lvl + d).max(0);
        }
        lvl
    }

    /// The structural bracket events in offset order: `(offset, delta)`
    /// where `delta` is `+1` for a structural `[` and `-1` for a `]`.
    /// Brackets inside inert spans are not recorded.
    #[must_use]
    pub fn events(&self) -> &[(u32, i32)] {
        &self.events
    }

    /// Inert byte ranges where `[` / `]` are literal: `(start, end,
    /// terminated)`. `terminated` is `false` for a span that runs to EOF.
    #[must_use]
    pub fn inert_spans(&self) -> &[(u32, u32, bool)] {
        &self.inert
    }

    /// **The recovery decision, from the index alone.** `true` when
    /// inserting a single `]` at byte offset `off` makes the source
    /// bracket-balanced (zero unterminated `[`), computed via a prefix
    /// sum + sparse forward walk — no re-lex.
    #[must_use]
    pub fn close_bracket_balances(&self, off: u32) -> bool {
        if off > self.len || self.inert_for_insert(off) {
            return false;
        }
        let mut lvl = 0i32;
        let mut inserted = false;
        for &(o, d) in &self.events {
            if !inserted && o >= off {
                // The inserted `]` lands here: a closer at level 0
                // closes nothing (clamp), matching stack semantics.
                lvl = (lvl - 1).max(0);
                inserted = true;
            }
            lvl = (lvl + d).max(0);
        }
        if !inserted {
            lvl = (lvl - 1).max(0);
        }
        lvl == 0
    }
}

/// Scanner state. Separated from [`BracketIndex`] so the recursive
/// command-sub sub-scan can borrow it mutably.
struct Builder<'a> {
    source: &'a str,
    bytes: &'a [u8],
    config: crate::LexerConfig,
    events: Vec<(u32, i32)>,
    inert: Vec<(u32, u32, bool)>,
}

impl Builder<'_> {
    fn push_inert(&mut self, start: usize, end: usize, terminated: bool) {
        if end > start {
            self.inert.push((
                u32::try_from(start).expect("offset fits u32"),
                u32::try_from(end).expect("offset fits u32"),
                terminated,
            ));
        }
    }

    fn push_event(&mut self, off: usize, delta: i32) {
        self.events
            .push((u32::try_from(off).expect("offset fits u32"), delta));
    }

    /// Consume a command-position comment, including any lines continued by
    /// a backslash-newline, and mark its bytes inert for bracket recovery.
    /// The terminating unescaped newline is left for the caller so it can
    /// restore command position, matching `Lexer::parse_comment`.
    fn scan_comment_inert(&mut self, start: usize) -> usize {
        let n = self.bytes.len();
        let mut i = start;
        while i < n {
            match self.bytes[i] {
                b'\n' => break,
                b'\\' => {
                    // Consume the canonical Tcl escape unit. This is what
                    // distinguishes an even run (`\\\\\n`, whose newline
                    // ends the comment) from an odd run. Raw escaped CR ends
                    // before a following LF, which still ends the comment.
                    i = backslash_escape_end(self.source, i);
                }
                _ => i += 1,
            }
        }
        self.push_inert(start, i, true);
        i
    }

    /// Top-level / quoted script scan: word-based brace words, quote
    /// toggling, command-position comments, escapes, `${…}`, and `[…]` command substitutions
    /// (which recurse into [`Self::scan_cmd_sub`]).
    fn scan_top(&mut self) {
        let n = self.bytes.len();
        let mut i = 0usize;
        // `is_newword`: the previous emitted token was Sep / Eol / Str /
        // Expand. Source start is a word boundary (initial last_kind =
        // Eol).
        let mut newword = true;
        let mut in_quote = false;
        let mut command_start = true;
        while i < n {
            if self.bytes[i] == b'#' && command_start && !in_quote {
                i = self.scan_comment_inert(i);
                newword = true;
                continue;
            }
            match self.bytes[i] {
                b'\\' => {
                    // Escape pair (lone `\` at EOF: 1 byte). Inert for
                    // brackets.  Escape pairs are always terminated
                    // (even when truncated at EOF, insertion past them
                    // is structural).
                    let end = (i + 2).min(n);
                    let continuation = is_line_continuation(self.source, i);
                    self.push_inert(i, end, true);
                    // A backslash-newline (`\<newline>`) is a line continuation
                    // that acts as a word separator (it collapses to a space),
                    // so a `{`/`"` on the continued line still opens a word.
                    // Every other `\X` escape is part of the current word.
                    newword = !in_quote && continuation;
                    if !newword {
                        command_start = false;
                    }
                    i = end;
                }
                b'\n' | b';' if !in_quote => {
                    i += 1;
                    newword = true;
                    command_start = true;
                }
                b' ' | b'\t' | b'\r' | 0x0b | 0x0c if !in_quote => {
                    i += 1;
                    newword = true;
                }
                b'{' if newword && !in_quote => {
                    // Verbatim brace word — the whole `{…}` span is
                    // inert for brackets.  STR keeps `newword` true.
                    let (end, terminated) = scan_brace_word(self.bytes, i);
                    self.push_inert(i, end, terminated);
                    i = end;
                    newword = true;
                    command_start = false;
                }
                b'"' if newword && !in_quote => {
                    in_quote = true;
                    i += 1;
                    newword = false;
                    command_start = false;
                }
                b'"' if in_quote => {
                    in_quote = false;
                    i += 1;
                    newword = false;
                    command_start = false;
                }
                b'$' if self.bytes.get(i + 1) == Some(&b'{') => {
                    let (end, terminated) =
                        scan_dollar_brace(self.bytes, i, self.config.braced_var);
                    self.push_inert(i, end, terminated);
                    i = end;
                    newword = false;
                    command_start = false;
                }
                b'[' => {
                    self.push_event(i, 1);
                    let (next_i, _) = self.scan_cmd_sub(i + 1, 0);
                    i = next_i;
                    newword = false;
                    command_start = false;
                }
                b']' => {
                    self.push_event(i, -1);
                    i += 1;
                    newword = false;
                    command_start = false;
                }
                _ => {
                    i += 1;
                    newword = false;
                    command_start = false;
                }
            }
        }
    }

    /// Scan a `[…]` command-substitution interior starting at `start`
    /// (just past the `[`), word by word as
    /// [`crate::ranges::command_substitution_end`] reads it: a `{` or `"`
    /// opens a braced or quoted word only at the start of a word, a braced
    /// word is one inert span, a `#` at command position starts an inert
    /// comment, and a `]` outside a quoted word closes the substitution;
    /// nested `[` recurse. Records nested structural bracket events and
    /// inert spans for brace words / escapes / `${…}`. Returns (offset,
    /// terminated): the offset just past the closing `]` (terminated), or
    /// EOF (unterminated).
    ///
    /// `depth` is the nesting level of this call (0 at the top, from
    /// [`Builder::scan_top`]); past [`MAX_NESTED_BRACKET_DEPTH`] a nested
    /// `[` is scanned as an ordinary character rather than recursed into.
    fn scan_cmd_sub(&mut self, start: usize, depth: u32) -> (usize, bool) {
        let n = self.bytes.len();
        let mut i = start;
        let mut in_quotes = false;
        let mut command_start = true;
        let mut newword = true;
        let past_cap = MAX_NESTED_BRACKET_DEPTH.exceeded(depth);
        while i < n {
            if self.bytes[i] == b'#' && command_start && !in_quotes {
                i = self.scan_comment_inert(i);
                continue;
            }
            match self.bytes[i] {
                b'"' if in_quotes || newword => {
                    // A closing quote ends the word; what follows starts the
                    // next one, as the shared scanner reads it.
                    newword = in_quotes;
                    in_quotes = !in_quotes;
                    command_start = false;
                    i += 1;
                }
                b'[' if !past_cap => {
                    self.push_event(i, 1);
                    let (next_i, _) = self.scan_cmd_sub(i + 1, depth + 1);
                    i = next_i;
                    newword = false;
                    command_start = false;
                }
                b']' if !in_quotes => {
                    self.push_event(i, -1);
                    return (i + 1, true); // terminated
                }
                b'\\' => {
                    let end = (i + 2).min(n);
                    let continuation = is_line_continuation(self.source, i);
                    self.push_inert(i, end, true); // escape pair, always terminated
                    i = end;
                    newword = !in_quotes && continuation;
                    if !continuation {
                        command_start = false;
                    }
                }
                b'$' if self.bytes.get(i + 1) == Some(&b'{') => {
                    let (end, terminated) =
                        scan_dollar_brace(self.bytes, i, self.config.braced_var);
                    self.push_inert(i, end, terminated);
                    i = end;
                    newword = false;
                    command_start = false;
                }
                b'{' if newword && !in_quotes => {
                    // A brace word: the whole `{…}` is inert for brackets.
                    let (end, terminated) = scan_brace_word(self.bytes, i);
                    self.push_inert(i, end, terminated);
                    i = end;
                    newword = true;
                    command_start = false;
                }
                b'\n' | b';' if !in_quotes => {
                    command_start = true;
                    newword = true;
                    i += 1;
                }
                b' ' | b'\t' | b'\r' | 0x0b | 0x0c if !in_quotes => {
                    newword = true;
                    i += 1;
                }
                _ => {
                    newword = false;
                    command_start = false;
                    i += 1;
                }
            }
        }
        (n, false) // unterminated
    }
}

/// Scan a verbatim brace word starting at the opening `{` at `start`.
/// Counts nested `{` / `}` (a `\}` does not close — the pair is
/// consumed). Returns the offset just past the matching `}`, or EOF if
/// unterminated.
fn scan_brace_word(bytes: &[u8], start: usize) -> (usize, bool) {
    let n = bytes.len();
    let mut i = start + 1; // skip `{`
    let mut level: u32 = 1;
    while i < n {
        match bytes[i] {
            b'\\' => {
                i = (i + 2).min(n); // skip the escaped pair
            }
            b'{' => {
                level += 1;
                i += 1;
            }
            b'}' => {
                level -= 1;
                i += 1;
                if level == 0 {
                    return (i, true); // terminated
                }
            }
            _ => i += 1,
        }
    }
    (n, false) // unterminated
}

/// Scan an original braced variable reference under the selected close rule.
/// The shared variable-name owner supplies its exact closer; no local Tcl9
/// brace-counting parser can override a retained Tcl8 source configuration.
fn scan_dollar_brace(
    bytes: &[u8],
    start: usize,
    style: tcl_dialect::BracedVarStyle,
) -> (usize, bool) {
    match crate::braced_var_name_end(bytes, start + 2, style) {
        crate::BracedVarEnd::Closed(end) => (end + 1, true),
        crate::BracedVarEnd::Unterminated => (bytes.len(), false),
    }
}

// Brace dimension (`{` / `}`) — the second script sublanguage the doc names
// alongside brackets. Same methodology and decision procedure; the *rules*
// differ: a `{` opens a brace word only at a word boundary (mid-word `{` is
// literal — `a{` is complete in C Tcl 9.0.3), inside a brace word nesting is
// verbatim (`\}` does not close), and a `#` at command start begins a comment
// whose braces are ignored. Quotes make braces literal; command-substitution
// interiors count braces (`[set x {` is incomplete).

/// A structural-state index for the script brace (`{` / `}`) dimension,
/// captured in a single forward scan. See the module header; the decision
/// procedure mirrors [`BracketIndex`].
#[derive(Debug, Clone)]
pub struct BraceIndex {
    /// Structural brace events in ascending offset order: `+1` for a
    /// structural `{`, `-1` for a structural `}`.
    events: Vec<(u32, i32)>,
    /// Inert byte ranges where `{` / `}` are literal (comments, quoted
    /// runs, escape pairs, `${…}`).  `(start, end, terminated)` — see
    /// [`BracketIndex::inert`] for the unterminated-span semantics.
    /// Sorted, merged.
    inert: Vec<(u32, u32, bool)>,
    /// Source length in bytes.
    len: u32,
}

impl BraceIndex {
    /// Build the index for `source` (one forward scan).
    #[must_use]
    pub fn build(source: &str) -> Self {
        Self::build_with_config(source, crate::LexerConfig::default())
    }

    /// Build source structural geometry with the actual retained lexical axes.
    /// The braced-variable inertness follows this configuration. Existing
    /// structural recovery approximations remain independent of tokenisation,
    /// command binding and semantic execution.
    #[must_use]
    pub fn build_with_config(source: &str, config: crate::LexerConfig) -> Self {
        let bytes = source.as_bytes();
        let mut b = BraceBuilder {
            bytes,
            config,
            events: Vec::new(),
            inert: Vec::new(),
        };
        b.scan_top();
        b.inert.sort_unstable();
        let mut merged: Vec<(u32, u32, bool)> = Vec::with_capacity(b.inert.len());
        for (s, e, term) in b.inert {
            match merged.last_mut() {
                Some(last) if s <= last.1 => {
                    last.1 = last.1.max(e);
                    last.2 = last.2 || term;
                }
                _ => merged.push((s, e, term)),
            }
        }
        BraceIndex {
            events: b.events,
            inert: merged,
            len: u32::try_from(bytes.len()).expect("source length fits u32"),
        }
    }

    /// `true` when inserting a `}` *at* `off` would be absorbed as a
    /// literal.  Unterminated spans at EOF swallow insertion at EOF
    /// (same semantics as [`BracketIndex::inert_for_insert`]).
    #[must_use]
    fn inert_for_insert(&self, off: u32) -> bool {
        let idx = self.inert.partition_point(|&(s, _, _)| s < off);
        if idx == 0 {
            return false;
        }
        let (start, end, terminated) = self.inert[idx - 1];
        if !terminated && end == self.len {
            start < off && off <= end
        } else {
            start < off && off < end
        }
    }

    /// Number of unterminated `{` at EOF — the final clamped brace level.
    #[must_use]
    pub fn unterminated_count(&self) -> i32 {
        let mut lvl = 0i32;
        for &(_, d) in &self.events {
            lvl = (lvl + d).max(0);
        }
        lvl
    }

    /// The structural brace events in offset order: `(offset, delta)`
    /// where `delta` is `+1` for a structural `{` and `-1` for a `}`.
    #[must_use]
    pub fn events(&self) -> &[(u32, i32)] {
        &self.events
    }

    /// Inert byte ranges where `{` / `}` are literal: `(start, end,
    /// terminated)`.
    #[must_use]
    pub fn inert_spans(&self) -> &[(u32, u32, bool)] {
        &self.inert
    }

    /// `true` when inserting a single `}` at byte offset `off` makes the
    /// source brace-balanced (zero unterminated `{`), from the index
    /// alone (prefix + forward walk).
    #[must_use]
    pub fn close_brace_balances(&self, off: u32) -> bool {
        if off > self.len || self.inert_for_insert(off) {
            return false;
        }
        let mut lvl = 0i32;
        let mut inserted = false;
        for &(o, d) in &self.events {
            if !inserted && o >= off {
                lvl = (lvl - 1).max(0);
                inserted = true;
            }
            lvl = (lvl + d).max(0);
        }
        if !inserted {
            lvl = (lvl - 1).max(0);
        }
        lvl == 0
    }
}

/// Brace-dimension scanner state.
struct BraceBuilder<'a> {
    bytes: &'a [u8],
    config: crate::LexerConfig,
    events: Vec<(u32, i32)>,
    inert: Vec<(u32, u32, bool)>,
}

impl BraceBuilder<'_> {
    fn push_inert(&mut self, start: usize, end: usize, terminated: bool) {
        if end > start {
            self.inert.push((
                u32::try_from(start).expect("offset fits u32"),
                u32::try_from(end).expect("offset fits u32"),
                terminated,
            ));
        }
    }

    fn push_event(&mut self, off: usize, delta: i32) {
        self.events
            .push((u32::try_from(off).expect("offset fits u32"), delta));
    }

    /// Build the index by scanning `source` as a top-level script.
    fn scan_top(&mut self) {
        let _ = self.scan_script(0, false, 0);
    }

    /// Scan a script region for the brace dimension, recursively. A
    /// `[…]` command-substitution interior **is itself a script** in C
    /// Tcl 9.0.3 — `info complete` parses it with the same word-based
    /// brace + extra-chars rules — so the same routine handles both the
    /// top level (`stop_at_bracket = false`, runs to EOF) and a
    /// command-sub interior (`stop_at_bracket = true`, returns past the
    /// matching `]`). This is the doc's "parse `[…]` interiors as nested
    /// scripts, not count braces".
    ///
    /// Returns `(end_offset, terminal)`. `terminal` is `true` when an
    /// "extra characters after close-brace/quote" error fired — a
    /// terminal parse error C Tcl reports as *complete* — which
    /// propagates up through every enclosing scope so the whole input is
    /// treated as complete (matching `[set x {b}{`).
    /// Advance one byte inside a verbatim brace word (`brace_level > 0`),
    /// emitting brace events / inert escape pairs and returning the new
    /// offset. Identical semantics to the inline brace arm it was
    /// extracted from: nested `{` / `}` count, `\}` is escaped (does not
    /// close), and the outermost close resets the word flags.
    fn step_brace_word(
        &mut self,
        i: usize,
        brace_level: &mut u32,
        command_start: &mut bool,
        newword: &mut bool,
        just_closed_word: &mut bool,
    ) -> usize {
        let n = self.bytes.len();
        match self.bytes[i] {
            b'\\' => {
                let end = (i + 2).min(n);
                self.push_inert(i, end, true); // escape pair, always terminated
                end
            }
            b'{' => {
                *brace_level += 1;
                self.push_event(i, 1);
                i + 1
            }
            b'}' => {
                *brace_level -= 1;
                self.push_event(i, -1);
                if *brace_level == 0 {
                    *newword = true;
                    *command_start = false;
                    *just_closed_word = true;
                }
                i + 1
            }
            _ => i + 1,
        }
    }

    /// Consume a `#` comment starting at `i` (the `#`), pushing it as one
    /// inert (always-terminated) range and returning the offset of the
    /// terminating `\n` (or EOF). Identical to the inline comment arm.
    fn scan_comment_inert(&mut self, i: usize) -> usize {
        let n = self.bytes.len();
        let mut j = i;
        while j < n && self.bytes[j] != b'\n' {
            j += 1;
        }
        self.push_inert(i, j, true); // comment, always terminated
        j
    }

    /// Whether the byte at `i` legitimately follows a closed brace/quote word.
    fn separates_closed_word(&self, i: usize, stop_at_bracket: bool) -> bool {
        match self.bytes[i] {
            b' ' | b'\t' | b'\r' | b'\n' | b';' => true,
            b'\\' => backslash_continuation_end(self.bytes, i).is_some(),
            // A `]` terminating the enclosing command sub is a
            // legitimate follow (`[set x {b}]`), not extra-chars.
            b']' => stop_at_bracket,
            _ => false,
        }
    }

    fn scan_script(&mut self, start: usize, stop_at_bracket: bool, depth: u32) -> (usize, bool) {
        let n = self.bytes.len();
        let mut i = start;
        // `command_start`: a `#` here begins a comment (only at the start
        // of a command — after Eol / `;` / scope start).
        let mut command_start = true;
        // `is_newword`: a `{` / `"` here is a delimiter-word opener.
        let mut newword = true;
        // Verbatim brace-word nesting depth (0 = command/word context).
        let mut brace_level: u32 = 0;
        let past_cap = MAX_NESTED_BRACKET_DEPTH.exceeded(depth);
        // Set right after a brace/quote *word* closes; the next char
        // decides separator (normal) vs terminal "extra characters".
        let mut just_closed_word = false;
        while i < n {
            if brace_level > 0 {
                i = self.step_brace_word(
                    i,
                    &mut brace_level,
                    &mut command_start,
                    &mut newword,
                    &mut just_closed_word,
                );
                continue;
            }
            // "Extra characters after close-brace/quote" gate: terminal
            // extra-chars stop and propagate "complete".
            if just_closed_word {
                if !self.separates_closed_word(i, stop_at_bracket) {
                    return (i, true);
                }
                just_closed_word = false;
            }
            // Command / word context.
            match self.bytes[i] {
                b']' if stop_at_bracket => return (i + 1, false),
                b'#' if command_start => i = self.scan_comment_inert(i),
                b'\\' => {
                    let end = (i + 2).min(n);
                    self.push_inert(i, end, true); // escape pair
                    if i + 1 < n && self.bytes[i + 1] == b'\n' {
                        // Line continuation (`\<newline>`) behaves like
                        // whitespace: a word separator that keeps the command
                        // open, so a `{`/`"` on the continued line still opens a
                        // word. `command_start` is preserved,
                        // as in the whitespace arm.
                        newword = true;
                    } else {
                        newword = false;
                        command_start = false;
                    }
                    i = end;
                }
                b'\n' | b';' => {
                    i += 1;
                    newword = true;
                    command_start = true;
                }
                b' ' | b'\t' | b'\r' => {
                    // Leading whitespace does not consume a word, so it
                    // preserves `command_start` — ` # foo` is a comment.
                    i += 1;
                    newword = true;
                }
                b'{' if newword => {
                    brace_level = 1;
                    self.push_event(i, 1);
                    i += 1;
                    command_start = false;
                }
                b'"' if newword => {
                    let (end, terminal) = self.scan_quoted(i + 1, depth);
                    if terminal {
                        return (end, true);
                    }
                    i = end;
                    newword = false;
                    command_start = false;
                    just_closed_word = true;
                }
                b'$' if self.bytes.get(i + 1) == Some(&b'{') => {
                    let (end, terminated) =
                        scan_dollar_brace(self.bytes, i, self.config.braced_var);
                    if !terminated {
                        self.push_event(i + 1, 1); // unterminated `${`
                    }
                    self.push_inert(i, end, terminated);
                    i = end;
                    newword = false;
                    command_start = false;
                }
                b'[' if !past_cap => {
                    let (end, terminal) = self.scan_script(i + 1, true, depth + 1);
                    if terminal {
                        return (end, true);
                    }
                    i = end;
                    newword = false;
                    command_start = false;
                }
                b'}' => {
                    // Extra/literal close with nothing open: clamps in
                    // `unterminated_count` (`}{` is complete in C Tcl).
                    self.push_event(i, -1);
                    i += 1;
                    newword = false;
                    command_start = false;
                }
                _ => {
                    i += 1;
                    newword = false;
                    command_start = false;
                }
            }
        }
        (n, false)
    }

    /// Scan a `"…"` quoted run (the opening `"` already consumed).
    /// Braces inside are literal, but command substitutions inside are
    /// still scripts (their braces count). Returns `(end, terminal)` —
    /// `end` is past the closing `"` (or EOF); `terminal` propagates an
    /// extra-chars error from a nested command sub.
    fn scan_quoted(&mut self, start: usize, depth: u32) -> (usize, bool) {
        let n = self.bytes.len();
        let mut i = start;
        let past_cap = MAX_NESTED_BRACKET_DEPTH.exceeded(depth);
        while i < n {
            match self.bytes[i] {
                b'\\' => i = (i + 2).min(n),
                b'"' => return (i + 1, false),
                b'[' if !past_cap => {
                    let (end, terminal) = self.scan_script(i + 1, true, depth + 1);
                    if terminal {
                        return (end, true);
                    }
                    i = end;
                }
                b'$' if self.bytes.get(i + 1) == Some(&b'{') => {
                    let (end, _) = scan_dollar_brace(self.bytes, i, self.config.braced_var);
                    i = end;
                }
                _ => i += 1,
            }
        }
        (n, false)
    }
}

// Expr paren dimension (`(` / `)`) — the doc's third structural index. Parens
// nest in expressions where they don't at script level, and the opaque tokens
// `[…]` / `"…"` / `{…}` / `${…}` / `$arr(idx)` are whole tokens whose interior
// parens never count. The Rust expr lexer already tokenises exactly that way
// (`$arr(idx)` is one `Variable`; strings / command subs are whole tokens), so
// the paren index is built **directly from the lexer's token stream** — the
// most faithful possible "store the lexer's entry-state per token".

/// The paren-balance verdict of an expression, matching C Tcl 9.0.3
/// `expr`'s paren diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParenBalance {
    /// Every `(` is matched and no `)` precedes its opener.
    Balanced,
    /// At least one unmatched `(` remains at EOF (C Tcl: "unbalanced
    /// open paren").
    OpenHeavy,
    /// A `)` appears with no open `(` (C Tcl: "unbalanced close paren").
    CloseHeavy,
}

/// A structural-state index for the expr paren (`(` / `)`) dimension,
/// built from the expr lexer's token stream.
#[derive(Debug, Clone)]
pub struct ExprParenIndex {
    /// Grouping-paren events in ascending offset order (`+1` / `-1`).
    events: Vec<(u32, i32)>,
    /// Inert byte ranges `[start, end)` — the opaque expr tokens
    /// (`String` / `Variable` / `Command`) whose interior parens never
    /// count. Sorted, merged.
    inert: Vec<(u32, u32)>,
    /// Source length in bytes.
    len: u32,
}

impl ExprParenIndex {
    /// Build the index for an expression `source`.
    #[must_use]
    pub fn build(source: &str) -> Self {
        let tokens = crate::expr_lexer::tokenise_expr(source, None);
        let mut events = Vec::new();
        let mut inert = Vec::new();
        for t in &tokens {
            match t.kind {
                crate::expr_lexer::ExprTokenType::ParenOpen => events.push((t.start, 1)),
                crate::expr_lexer::ExprTokenType::ParenClose => events.push((t.start, -1)),
                // Opaque whole tokens: their interior parens are inert.
                // `end` is inclusive, so the half-open span ends at
                // `end + 1`.
                crate::expr_lexer::ExprTokenType::String
                | crate::expr_lexer::ExprTokenType::Variable
                | crate::expr_lexer::ExprTokenType::Command => {
                    inert.push((t.start, t.end.saturating_add(1)));
                }
                _ => {}
            }
        }
        // The lexer emits tokens left-to-right, so both vectors are
        // already sorted; merge inert overlaps defensively.
        inert.sort_unstable();
        let mut merged: Vec<(u32, u32)> = Vec::with_capacity(inert.len());
        for (s, e) in inert {
            match merged.last_mut() {
                Some(last) if s <= last.1 => last.1 = last.1.max(e),
                _ => merged.push((s, e)),
            }
        }
        ExprParenIndex {
            events,
            inert: merged,
            len: u32::try_from(source.len()).expect("source length fits u32"),
        }
    }

    /// The paren-balance verdict (signed running level: a negative
    /// running minimum is a close before its opener; a positive final
    /// level is unmatched opens).
    #[must_use]
    pub fn balance(&self) -> ParenBalance {
        let mut lvl = 0i32;
        let mut min = 0i32;
        for &(_, d) in &self.events {
            lvl += d;
            min = min.min(lvl);
        }
        if min < 0 {
            ParenBalance::CloseHeavy
        } else if lvl > 0 {
            ParenBalance::OpenHeavy
        } else {
            ParenBalance::Balanced
        }
    }

    /// Number of unmatched `(` at EOF (the clamped open level).
    #[must_use]
    pub fn unmatched_opens(&self) -> i32 {
        let mut lvl = 0i32;
        for &(_, d) in &self.events {
            lvl = (lvl + d).max(0);
        }
        lvl
    }

    /// `true` when inserting a `)` *at* `off` would be absorbed by an
    /// opaque token (strictly inside an inert span).
    #[must_use]
    fn inert_for_insert(&self, off: u32) -> bool {
        let idx = self.inert.partition_point(|&(s, _)| s < off);
        if idx == 0 {
            return false;
        }
        let (start, end) = self.inert[idx - 1];
        start < off && off < end
    }

    /// `true` when inserting a single `)` at byte offset `off` makes the
    /// expression paren-balanced, from the index alone.
    #[must_use]
    pub fn close_paren_balances(&self, off: u32) -> bool {
        if off > self.len || self.inert_for_insert(off) {
            return false;
        }
        let mut lvl = 0i32;
        let mut inserted = false;
        for &(o, d) in &self.events {
            if !inserted && o >= off {
                lvl = (lvl - 1).max(0);
                inserted = true;
            }
            lvl = (lvl + d).max(0);
        }
        if !inserted {
            lvl = (lvl - 1).max(0);
        }
        lvl == 0
    }
}

// Unified script completeness — a faithful `Tcl_CommandComplete` port built on
// the recursive-script scanner. Answers "does this string form one or more
// *complete* commands, or is more input needed?" exactly like `info complete`,
// combining the brace / bracket / quote dimensions plus comments, `${…}`,
// escapes, the terminal "extra characters after close" rule, and trailing
// line-continuation. This is the primitive an incremental reparser needs to
// decide whether an edited region closes cleanly.

/// Outcome of scanning a script region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Completeness {
    /// Reached the terminator (`]`) or a clean EOF with nothing open.
    Closed,
    /// Reached EOF with an unclosed brace / bracket / quote / `${`.
    Incomplete,
    /// A terminal "extra characters after close-brace/quote" parse error
    /// — C Tcl reports such a string as *complete*.
    Terminal,
}

/// `true` when `source` is a complete Tcl script (every command is
/// complete; no more input is needed) — a
/// / `info complete`, verified against C Tcl 9.0.3.
#[must_use]
pub fn script_is_complete(source: &str) -> bool {
    let b = source.as_bytes();
    match scan_complete(source, b, 0, false, 0).1 {
        Completeness::Incomplete => false,
        Completeness::Terminal => true,
        // A clean parse is still incomplete if it ends on a
        // backslash-newline line continuation (`a\<nl>`).
        Completeness::Closed => !ends_with_line_continuation(b),
    }
}

/// Where each chunk `tclsh` evaluates from its stdin ends: it reads lines
/// until the lines read form complete commands (`Tcl_CommandComplete`) and
/// evaluates them. An incomplete last chunk never runs, so has no end here.
#[must_use]
pub fn stdin_chunk_ends(src: &[u8]) -> Vec<usize> {
    // Completeness reads only the ASCII structure: each other byte stands as
    // one letter, keeping every offset.
    let text: String = src
        .iter()
        .map(|&byte| {
            if byte.is_ascii() {
                char::from(byte)
            } else {
                'a'
            }
        })
        .collect();
    let line_ends = text
        .match_indices('\n')
        .map(|(at, _)| at + 1)
        .chain(std::iter::once(text.len()));
    let mut ends = Vec::new();
    let mut start = 0;
    for end in line_ends {
        if end > start && script_is_complete(&text[start..end]) {
            ends.push(end);
            start = end;
        }
    }
    ends
}

/// `true` when `b` ends with a backslash-newline continuation: an odd
/// run of backslashes immediately before the final LF.
pub(crate) fn ends_with_line_continuation(b: &[u8]) -> bool {
    let mut k = b.len();
    if k == 0 || b[k - 1] != b'\n' {
        return false;
    }
    k -= 1; // the `\n`
    let mut backslashes = 0usize;
    while k > 0 && b[k - 1] == b'\\' {
        backslashes += 1;
        k -= 1;
    }
    backslashes % 2 == 1
}

/// Scan a script region for completeness, recursively (mirrors the brace
/// scanner's `scan_script`, but tracking every dimension). A `[…]`
/// interior is itself a script (`stop_at_bracket = true`). Returns
/// `(end_offset, Completeness)`.
/// Skip a `#` comment starting at `start` (the `#` itself), returning the
/// offset just past it. A trailing odd-backslash run before a newline
/// continues the comment onto the next line (it swallows it), matching
/// Tcl's backslash-newline-in-comment rule.
///
/// Documented bounded edge: a comment whose backslash-newline
/// continuation is left *pending* at EOF (e.g. `# c\<newline>`) is
/// reported complete by the caller but incomplete by C Tcl 9.0.3.
/// `Tcl_CommandComplete`'s character-level backslash-newline handling in
/// comments has subtle EOF semantics that depend on whether the dangling
/// line carries content; reproducing it exactly is not worth the
/// complexity for a case real Tcl never emits. Quantified at < 0.05% of
/// an adversarial fuzz corpus by
/// `ctcl9_script_is_complete_iff_info_complete`.
fn skip_comment(source: &str, start: usize) -> usize {
    let b = source.as_bytes();
    let n = b.len();
    let mut i = start;
    while i < n {
        match b[i] {
            b'\n' => break,
            // Consume one canonical Tcl escape at a time.  Besides avoiding
            // UTF-8 byte slicing, this naturally distinguishes even and odd
            // backslash runs and treats CRLF as one continued newline.
            b'\\' => i = backslash_escape_end(source, i),
            _ => i += 1,
        }
    }
    i
}

/// Advance one byte inside a verbatim brace word (`brace_level > 0`),
/// returning the new offset. Identical semantics to the inline brace arm
/// it was extracted from; updates `brace_level` and the word flags on the
/// outermost close. `brace_is_var` is `true` for a `${name}` variable
/// brace, which — unlike a command/argument brace word — *continues the
/// word* on close (no "extra characters" terminal): `${a}b` is complete
/// and keeps parsing, whereas `{a}b` does not.
fn step_brace_word(
    b: &[u8],
    i: usize,
    brace_level: &mut u32,
    command_start: &mut bool,
    newword: &mut bool,
    just_closed_word: &mut bool,
    brace_is_var: bool,
) -> usize {
    let n = b.len();
    match b[i] {
        b'\\' => (i + 2).min(n),
        b'{' => {
            *brace_level += 1;
            i + 1
        }
        b'}' => {
            *brace_level -= 1;
            if *brace_level == 0 {
                *command_start = false;
                if brace_is_var {
                    *newword = false;
                } else {
                    *newword = true;
                    *just_closed_word = true;
                }
            }
            i + 1
        }
        _ => i + 1,
    }
}

fn scan_complete(
    source: &str,
    b: &[u8],
    start: usize,
    stop_at_bracket: bool,
    depth: u32,
) -> (usize, Completeness) {
    let n = b.len();
    let mut i = start;
    let mut command_start = true;
    let mut newword = true;
    let mut brace_level: u32 = 0;
    let mut just_closed_word = false;
    let mut brace_is_var = false;
    let past_cap = MAX_NESTED_BRACKET_DEPTH.exceeded(depth);
    while i < n {
        if brace_level > 0 {
            i = step_brace_word(
                b,
                i,
                &mut brace_level,
                &mut command_start,
                &mut newword,
                &mut just_closed_word,
                brace_is_var,
            );
            continue;
        }
        if just_closed_word {
            match b[i] {
                b' ' | b'\t' | b'\r' | b'\n' | b';' => just_closed_word = false,
                b'\\' if backslash_continuation_end(b, i).is_some() => {
                    just_closed_word = false;
                }
                b']' if stop_at_bracket => just_closed_word = false,
                _ => return (i, Completeness::Terminal),
            }
        }
        match b[i] {
            b']' if stop_at_bracket => return (i + 1, Completeness::Closed),
            b'#' if command_start => i = skip_comment(source, i),
            // A backslash-newline is a line continuation (acts as
            // whitespace -> word boundary, same command); any other
            // escaped char is content.
            b'\\' if let Some(end) = backslash_continuation_end(b, i) => {
                i = end;
                newword = true;
            }
            b'\\' => {
                i = (i + 2).min(n);
                newword = false;
                command_start = false;
            }
            b'\n' | b';' => {
                i += 1;
                newword = true;
                command_start = true;
            }
            b' ' | b'\t' | b'\r' => {
                i += 1;
                newword = true;
            }
            b'{' if newword => {
                brace_level = 1;
                brace_is_var = false;
                i += 1;
                command_start = false;
            }
            b'"' if newword => match scan_complete_quoted(source, b, i + 1, depth) {
                (_, Completeness::Terminal) => return (n, Completeness::Terminal),
                (_, Completeness::Incomplete) => return (n, Completeness::Incomplete),
                (end, Completeness::Closed) => {
                    i = end;
                    newword = false;
                    command_start = false;
                    just_closed_word = true;
                }
            },
            b'$' if b.get(i + 1) == Some(&b'{') => {
                // `${name}` is `$` + a nesting brace word.
                brace_level = 1;
                brace_is_var = true;
                i += 2; // skip `${`; the `{` is the brace-word opener
                command_start = false;
            }
            b'[' if !past_cap => match scan_complete(source, b, i + 1, true, depth + 1) {
                (_, Completeness::Terminal) => return (n, Completeness::Terminal),
                (_, Completeness::Incomplete) => return (n, Completeness::Incomplete),
                (end, Completeness::Closed) => {
                    i = end;
                    newword = false;
                    command_start = false;
                }
            },
            _ => {
                i += 1;
                newword = false;
                command_start = false;
            }
        }
    }
    if brace_level > 0 || stop_at_bracket {
        // An open brace word, or a command-sub interior that reached EOF
        // without its closing `]`, is unterminated.
        (n, Completeness::Incomplete)
    } else {
        (n, Completeness::Closed)
    }
}

/// Scan a `"…"` quoted run (opening `"` consumed). Command subs inside
/// are scripts. `Closed` past the closing `"`, `Incomplete` at EOF
/// (unterminated quote), `Terminal` if a nested command sub errored.
fn scan_complete_quoted(source: &str, b: &[u8], start: usize, depth: u32) -> (usize, Completeness) {
    let n = b.len();
    let mut i = start;
    let past_cap = MAX_NESTED_BRACKET_DEPTH.exceeded(depth);
    while i < n {
        match b[i] {
            b'\\' => i = (i + 2).min(n),
            b'"' => return (i + 1, Completeness::Closed),
            b'[' if !past_cap => match scan_complete(source, b, i + 1, true, depth + 1) {
                (_, Completeness::Terminal) => return (n, Completeness::Terminal),
                (_, Completeness::Incomplete) => return (n, Completeness::Incomplete),
                (end, Completeness::Closed) => i = end,
            },
            b'$' if b.get(i + 1) == Some(&b'{') => {
                // `${name}` is `$` + a nesting brace word; if it never
                // closes, the quote (and script) is incomplete.
                let (end, closed) = scan_var_brace(b, i + 1);
                if !closed {
                    return (n, Completeness::Incomplete);
                }
                i = end;
            }
            _ => i += 1,
        }
    }
    (n, Completeness::Incomplete)
}

/// Scan a `${…}` variable-name brace word (the `{` is at `brace`),
/// nesting like a brace word (`\}` escaped). Returns `(end, closed)` —
/// `end` past the matching `}`, `closed` false if it ran to EOF.
fn scan_var_brace(b: &[u8], brace: usize) -> (usize, bool) {
    let n = b.len();
    let mut i = brace + 1;
    let mut level: u32 = 1;
    while i < n {
        match b[i] {
            b'\\' => i = (i + 2).min(n),
            b'{' => {
                level += 1;
                i += 1;
            }
            b'}' => {
                level -= 1;
                i += 1;
                if level == 0 {
                    return (i, true);
                }
            }
            _ => i += 1,
        }
    }
    (n, false)
}

// Incremental-reparse primitives. A cheap single byte-scan finds the top-level
// command boundaries (the stable split points an incremental reparser snaps
// edits to); `reparse_window` snaps a changed byte range outward to whole
// commands so only those need re-segmenting / re-analysing. Nesting (`{…}`,
// `${…}`, `"…"`, comments, escapes) is skipped by the same scanner that backs
// `script_is_complete`, and `[…]` by `ranges::command_substitution_end`, so
// the boundaries agree with the boundary owner without tokenising the whole
// document.
//
// Where the two differ is *leniency*: `script_is_complete` is an oracle and
// reports a terminal parse error, while boundary scanning must keep going the
// way the lexer does. Each arm below says which it takes and why. Registered
// as its own surface in `shared-utility-contracts-rust.md` (dialect-blind by
// construction) and pinned to `script::group_commands` over the corpora by
// `tcl-lexer`'s `differential_boundaries`.

/// The byte offsets that split `source` into top-level commands — each is
/// the position immediately after a top-level command terminator (`\n` or
/// `;`), plus `source.len()`. Separators inside braces / quotes / command
/// substitutions / comments do not split. Every returned offset `b`
/// satisfies `script_is_complete(&source[..b])` on well-formed input.
#[must_use]
pub fn command_boundaries(source: &str) -> Vec<u32> {
    let b = source.as_bytes();
    let n = b.len();
    let mut out: Vec<u32> = Vec::new();
    let mut i = 0usize;
    let mut command_start = true;
    let mut newword = true;
    let mut brace_level: u32 = 0;
    let mut brace_is_var = false;
    while i < n {
        if brace_level > 0 {
            match b[i] {
                b'\\' => i = (i + 2).min(n),
                b'{' => {
                    brace_level += 1;
                    i += 1;
                }
                b'}' => {
                    brace_level -= 1;
                    i += 1;
                    if brace_level == 0 {
                        command_start = false;
                        newword = !brace_is_var;
                    }
                }
                _ => i += 1,
            }
            continue;
        }
        match b[i] {
            b'#' if command_start => i = skip_comment(source, i),
            b'\\' if let Some(end) = backslash_continuation_end(b, i) => {
                i = end;
                newword = true;
            }
            b'\\' => {
                i = (i + 2).min(n);
                newword = false;
                command_start = false;
            }
            b'\n' | b';' => {
                i += 1;
                out.push(u32::try_from(i).expect("offset fits u32"));
                newword = true;
                command_start = true;
            }
            b' ' | b'\t' | b'\r' => {
                i += 1;
                newword = true;
            }
            b'{' if newword => {
                brace_level = 1;
                brace_is_var = false;
                i += 1;
                command_start = false;
            }
            b'"' if newword => {
                i = match scan_complete_quoted(source, b, i + 1, 0) {
                    // Same leniency as the `[` arm below: a nested `[…]`
                    // holding a weld C rejects makes the completeness
                    // scanner report `Terminal` with its offset collapsed
                    // to end-of-input, which would lose every later
                    // boundary. The quote's own delimiters are fine in
                    // that case, so take the close from the shared
                    // close-quote owner. Caught in tcllib's
                    // `page/util_quote.tcl` by `differential_boundaries`.
                    (_, Completeness::Terminal) => {
                        crate::ranges::close_quote_offset(source, i).map_or(n, |q| q + 1)
                    }
                    // `Closed` is past the `"`; `Incomplete` is `n`, which
                    // is right — an unterminated quote runs to EOF, and
                    // `scan_complete_quoted` (unlike the close-quote
                    // owner) knows the release's `${…}` nesting rule.
                    (end, _) => end,
                };
                newword = false;
                command_start = false;
            }
            b'$' if b.get(i + 1) == Some(&b'{') => {
                brace_level = 1;
                brace_is_var = true;
                i += 2;
                command_start = false;
            }
            // The `[…]` skip goes through the shared close-bracket owner
            // rather than `scan_complete`: the completeness scanner reports
            // `Terminal` for a weld C rejects (`{a}$b`), and its `[` arm
            // collapses that verdict's offset to end-of-input — so a nested
            // `[a [b {*}$c]]` (Tcl 9.0.4's `auto.tcl` writes exactly that)
            // would swallow every later boundary in the document. Boundary
            // scanning is lenient by contract, so it wants the lenient,
            // comment-aware, iterative closer instead of the completeness
            // oracle's error path. Caught by `differential_boundaries`'
            // coverage invariant.
            b'[' => {
                i = crate::ranges::command_substitution_end(source, i).unwrap_or(n);
                newword = false;
                command_start = false;
            }
            _ => {
                i += 1;
                newword = false;
                command_start = false;
            }
        }
    }
    if out.last() != Some(&u32::try_from(n).expect("offset fits u32")) {
        out.push(u32::try_from(n).expect("offset fits u32"));
    }
    out
}

/// Snap a changed byte range `[start, end)` outward to the smallest
/// window of **whole top-level commands** that contains it — the minimal
/// region an incremental reparser must re-segment / re-analyse. Returns
/// `(lo, hi)` byte offsets aligned to [`command_boundaries`] (or `0` /
/// `source.len()`).
#[must_use]
pub fn reparse_window(source: &str, start: u32, end: u32) -> (u32, u32) {
    let len = u32::try_from(source.len()).expect("offset fits u32");
    let start = start.min(len);
    let end = end.clamp(start, len);
    let bounds = command_boundaries(source);
    // `lo` = the greatest boundary <= start (commands before it are
    // unaffected); `hi` = the least boundary >= end.
    let lo = bounds.iter().copied().rfind(|&b| b <= start).unwrap_or(0);
    let hi = bounds.iter().copied().find(|&b| b >= end).unwrap_or(len);
    (lo, hi)
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod configured_variable_inertness_tests;
