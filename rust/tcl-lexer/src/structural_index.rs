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

//! **Experiment — structural-state recovery index (script bracket
//! dimension).**
//!
//! This module is the Rust prototype that validates the design in
//! `docs/design/compiler/error-recovery-rust-port.md` (synced from
//! main). It is the de-risking experiment the doc calls for: capture a
//! **structural-state index** in one scan, then answer *"does inserting
//! a closer at offset O balance the outermost open delimiter?"* from the
//! index alone — **no re-lex** — and prove the answer matches a real
//! re-scan.
//!
//! Scope: all three structural-index dimensions the doc names — the
//! script **`[`/`]` (bracket)** and **`{`/`}` (brace)** sublanguages
//! ([`BracketIndex`], [`BraceIndex`]) and the expr **`(`/`)` (paren)**
//! sublanguage ([`ExprParenIndex`]). It is deliberately *not wired into
//! production* — like the earlier prototypes (kept only in git history),
//! it instruments the model and adds no production value yet; the
//! incremental green-tree engine will build the productionised index.
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
//! ## The two corrections the prototypes forced (carried here)
//!
//! 1. **An extra closer closes nothing** — the running level clamps at
//!    0 (a `]` with nothing open is literal, not a negative level).
//! 2. **An unterminated opaque token reaching EOF makes the tail inert**
//!    — an unterminated `[…` / `"…` / `${…` swallows to EOF; here that
//!    falls out naturally (the open `[` stays open and a later inserted
//!    `]` closes *it*).
//!
//! ## Rust validation results (this module's `#[cfg(test)]` harness)
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
//! ### Findings the experiment surfaced (recorded for the port)
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
//!    hypothetical close — the doc's "command-sub interiors need care",
//!    confirmed and bounded in Rust.

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
        let bytes = source.as_bytes();
        let mut b = Builder {
            bytes,
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
    bytes: &'a [u8],
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

    /// Top-level / quoted script scan: word-based brace words, quote
    /// toggling, escapes, `${…}`, and `[…]` command substitutions
    /// (which recurse into [`Self::scan_cmd_sub`]).
    fn scan_top(&mut self) {
        let n = self.bytes.len();
        let mut i = 0usize;
        // `is_newword`: the previous emitted token was Sep / Eol / Str /
        // Expand. Source start is a word boundary (initial last_kind =
        // Eol).
        let mut newword = true;
        let mut in_quote = false;
        while i < n {
            match self.bytes[i] {
                b'\\' => {
                    // Escape pair (lone `\` at EOF: 1 byte). Inert for
                    // brackets.  Escape pairs are always terminated
                    // (even when truncated at EOF, insertion past them
                    // is structural).
                    let end = (i + 2).min(n);
                    self.push_inert(i, end, true);
                    // A backslash-newline (`\<newline>`) is a line continuation
                    // that acts as a word separator (it collapses to a space),
                    // so a `{`/`"` on the continued line still opens a word.
                    // Every other `\X` escape is part of the current word
                    // (RUST_ISSUE_088).
                    newword = !in_quote && i + 1 < n && self.bytes[i + 1] == b'\n';
                    i = end;
                }
                b' ' | b'\t' | b'\r' | b'\n' | b';' if !in_quote => {
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
                }
                b'"' if newword && !in_quote => {
                    in_quote = true;
                    i += 1;
                    newword = false;
                }
                b'"' if in_quote => {
                    in_quote = false;
                    i += 1;
                    newword = false;
                }
                b'$' if self.bytes.get(i + 1) == Some(&b'{') => {
                    let (end, terminated) = scan_dollar_brace(self.bytes, i);
                    self.push_inert(i, end, terminated);
                    i = end;
                    newword = false;
                }
                b'[' => {
                    self.push_event(i, 1);
                    let (next_i, _) = self.scan_cmd_sub(i + 1);
                    i = next_i;
                    newword = false;
                }
                b']' => {
                    self.push_event(i, -1);
                    i += 1;
                    newword = false;
                }
                _ => {
                    i += 1;
                    newword = false;
                }
            }
        }
    }

    /// Scan a `[…]` command-substitution interior starting at `start`
    /// (just past the `[`).'s
    /// **count-based** rules: `blevel` tracks `{` / `}` literally, a `]`
    /// closes only at `blevel == 0 && !in_quotes`, nested `[` / `]`
    /// recurse. Records nested structural bracket events and inert spans
    /// for brace interiors / escapes / `${…}`. Returns (offset,
    /// terminated): the offset just past the closing `]` (terminated), or
    /// EOF (unterminated).
    fn scan_cmd_sub(&mut self, start: usize) -> (usize, bool) {
        let n = self.bytes.len();
        let mut i = start;
        let mut blevel: u32 = 0;
        let mut in_quotes = false;
        // Start of the current inert brace run (when blevel transitions
        // 0 -> >0), so the whole `{…}` is one inert span.
        let mut brace_run_start: Option<usize> = None;
        while i < n {
            match self.bytes[i] {
                b'"' if blevel == 0 => {
                    in_quotes = !in_quotes;
                    i += 1;
                }
                b'[' if blevel == 0 && !in_quotes => {
                    self.push_event(i, 1);
                    let (next_i, _) = self.scan_cmd_sub(i + 1);
                    i = next_i;
                }
                b']' if blevel == 0 && !in_quotes => {
                    self.push_event(i, -1);
                    return (i + 1, true); // terminated
                }
                b'\\' => {
                    let end = (i + 2).min(n);
                    self.push_inert(i, end, true); // escape pair, always terminated
                    i = end;
                }
                b'$' if !in_quotes && blevel == 0 && self.bytes.get(i + 1) == Some(&b'{') => {
                    let (end, terminated) = scan_dollar_brace(self.bytes, i);
                    self.push_inert(i, end, terminated);
                    i = end;
                }
                b'{' if !in_quotes => {
                    if blevel == 0 {
                        brace_run_start = Some(i);
                    }
                    blevel += 1;
                    i += 1;
                }
                b'}' if !in_quotes => {
                    blevel = blevel.saturating_sub(1);
                    i += 1;
                    if blevel == 0
                        && let Some(s) = brace_run_start.take()
                    {
                        // Mark the closed `{…}` run inert for
                        // brackets (a `]` inside it never counted).
                        self.push_inert(s, i, true); // terminated
                    }
                }
                _ => {
                    i += 1;
                }
            }
        }
        // Unterminated to EOF. If a brace run was still open, the rest
        // is verbatim/inert; the open `[` stays open (its event was
        // pushed by the caller), so an inserted `]` closes it.
        if let Some(s) = brace_run_start {
            self.push_inert(s, n, false); // unterminated
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

/// Scan a `${…}` variable-name brace starting at the `$` at `start`.
/// Returns (offset just past the matching `}`, true) when terminated,
/// or (EOF, false) when unterminated. The interior is inert for brackets.
///
/// Matches `Lexer::parse_var`'s braced branch under the **Tcl 9** rule
/// (`BracedVarStyle::Tcl9Nesting`): inner `{…}` nests with brace counting
/// and `\X` is consumed as a literal pair, so the closer is the first `}`
/// at depth zero — `${a{b}c}` and `${a\}b}` both close at their final `}`.
/// The structural index is dialect-blind (its `build` API takes no config),
/// so 8.x sources with a `{`/`\` inside a `${…}` name — where the real
/// closer is the FIRST `}` — can mis-scan here; the effect is limited to
/// bracket/fold structure (inert spans), never token or diagnostic
/// semantics, which come from the dialect-aware lexer.
fn scan_dollar_brace(bytes: &[u8], start: usize) -> (usize, bool) {
    let n = bytes.len();
    let mut i = start + 2; // skip `${`
    let mut depth: u32 = 0;
    while i < n {
        match bytes[i] {
            b'}' if depth == 0 => return (i + 1, true), // terminated
            b'{' => depth += 1,
            b'}' => depth -= 1,
            // Skip the backslash; the loop step skips the escaped byte.
            b'\\' => i += 1,
            _ => {}
        }
        i += 1;
    }
    (n, false) // unterminated
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
        let bytes = source.as_bytes();
        let mut b = BraceBuilder {
            bytes,
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
        let _ = self.scan_script(0, false);
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

    fn scan_script(&mut self, start: usize, stop_at_bracket: bool) -> (usize, bool) {
        let n = self.bytes.len();
        let mut i = start;
        // `command_start`: a `#` here begins a comment (only at the start
        // of a command — after Eol / `;` / scope start).
        let mut command_start = true;
        // `is_newword`: a `{` / `"` here is a delimiter-word opener.
        let mut newword = true;
        // Verbatim brace-word nesting depth (0 = command/word context).
        let mut brace_level: u32 = 0;
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
            // "Extra characters after close-brace/quote" gate.
            if just_closed_word {
                match self.bytes[i] {
                    b' ' | b'\t' | b'\r' | b'\n' | b';' => just_closed_word = false,
                    b'\\' if matches!(self.bytes.get(i + 1), Some(b'\n' | b'\r')) => {
                        just_closed_word = false;
                    }
                    // A `]` terminating the enclosing command sub is a
                    // legitimate follow (`[set x {b}]`), not extra-chars.
                    b']' if stop_at_bracket => just_closed_word = false,
                    // Terminal extra-chars: stop and propagate "complete".
                    _ => return (i, true),
                }
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
                        // word (RUST_ISSUE_088). `command_start` is preserved,
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
                    let (end, terminal) = self.scan_quoted(i + 1);
                    if terminal {
                        return (end, true);
                    }
                    i = end;
                    newword = false;
                    command_start = false;
                    just_closed_word = true;
                }
                b'$' if self.bytes.get(i + 1) == Some(&b'{') => {
                    let (end, terminated) = scan_dollar_brace(self.bytes, i);
                    if !terminated {
                        self.push_event(i + 1, 1); // unterminated `${`
                    }
                    self.push_inert(i, end, terminated);
                    i = end;
                    newword = false;
                    command_start = false;
                }
                b'[' => {
                    let (end, terminal) = self.scan_script(i + 1, true);
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
    fn scan_quoted(&mut self, start: usize) -> (usize, bool) {
        let n = self.bytes.len();
        let mut i = start;
        while i < n {
            match self.bytes[i] {
                b'\\' => i = (i + 2).min(n),
                b'"' => return (i + 1, false),
                b'[' => {
                    let (end, terminal) = self.scan_script(i + 1, true);
                    if terminal {
                        return (end, true);
                    }
                    i = end;
                }
                b'$' if self.bytes.get(i + 1) == Some(&b'{') => {
                    let (end, _) = scan_dollar_brace(self.bytes, i);
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
    match scan_complete(b, 0, false).1 {
        Completeness::Incomplete => false,
        Completeness::Terminal => true,
        // A clean parse is still incomplete if it ends on a
        // backslash-newline line continuation (`a\<nl>`).
        Completeness::Closed => !ends_with_line_continuation(b),
    }
}

/// `true` when `b` ends with a backslash-newline continuation: an odd
/// run of `\` immediately before the final newline (optional `\r`).
fn ends_with_line_continuation(b: &[u8]) -> bool {
    let mut k = b.len();
    if k == 0 || b[k - 1] != b'\n' {
        return false;
    }
    k -= 1; // the `\n`
    if k > 0 && b[k - 1] == b'\r' {
        k -= 1; // the `\r` of a CRLF
    }
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
fn skip_comment(b: &[u8], start: usize) -> usize {
    let n = b.len();
    let mut i = start;
    loop {
        let line_start = i;
        while i < n && b[i] != b'\n' {
            i += 1;
        }
        let mut bs = 0usize;
        let mut k = i;
        while k > line_start && b[k - 1] == b'\\' {
            bs += 1;
            k -= 1;
        }
        if bs % 2 == 1 && i < n {
            i += 1; // swallow the escaped newline
            continue;
        }
        break;
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

fn scan_complete(b: &[u8], start: usize, stop_at_bracket: bool) -> (usize, Completeness) {
    let n = b.len();
    let mut i = start;
    let mut command_start = true;
    let mut newword = true;
    let mut brace_level: u32 = 0;
    let mut just_closed_word = false;
    let mut brace_is_var = false;
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
                b'\\' if matches!(b.get(i + 1), Some(b'\n' | b'\r')) => just_closed_word = false,
                b']' if stop_at_bracket => just_closed_word = false,
                _ => return (i, Completeness::Terminal),
            }
        }
        match b[i] {
            b']' if stop_at_bracket => return (i + 1, Completeness::Closed),
            b'#' if command_start => i = skip_comment(b, i),
            // A backslash-newline is a line continuation (acts as
            // whitespace -> word boundary, same command); any other
            // escaped char is content.
            b'\\' if matches!(b.get(i + 1), Some(b'\n' | b'\r')) => {
                i = (i + 2).min(n);
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
            b'"' if newword => match scan_complete_quoted(b, i + 1) {
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
            b'[' => match scan_complete(b, i + 1, true) {
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
fn scan_complete_quoted(b: &[u8], start: usize) -> (usize, Completeness) {
    let n = b.len();
    let mut i = start;
    while i < n {
        match b[i] {
            b'\\' => i = (i + 2).min(n),
            b'"' => return (i + 1, Completeness::Closed),
            b'[' => match scan_complete(b, i + 1, true) {
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
// commands so only those need re-segmenting / re-analysing. Nesting (`[…]`,
// `"…"`, `{…}`, `${…}`, comments, escapes) is skipped via the same recursive
// scanner that backs `script_is_complete`, so the boundaries agree with the
// segmenter without tokenising the whole document.

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
            b'#' if command_start => loop {
                let line_start = i;
                while i < n && b[i] != b'\n' {
                    i += 1;
                }
                let mut bs = 0usize;
                let mut k = i;
                while k > line_start && b[k - 1] == b'\\' {
                    bs += 1;
                    k -= 1;
                }
                if bs % 2 == 1 && i < n {
                    i += 1;
                    continue;
                }
                break;
            },
            b'\\' if matches!(b.get(i + 1), Some(b'\n' | b'\r')) => {
                i = (i + 2).min(n);
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
                i = scan_complete_quoted(b, i + 1).0;
                newword = false;
                command_start = false;
            }
            b'$' if b.get(i + 1) == Some(&b'{') => {
                brace_level = 1;
                brace_is_var = true;
                i += 2;
                command_start = false;
            }
            b'[' => {
                i = scan_complete(b, i + 1, true).0;
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
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    /// Ground truth: does inserting a `]` at `off` make `source`
    /// bracket-balanced, per a full re-scan (rebuild the index on the
    /// edited source)?
    fn ground_truth_balances(source: &str, off: usize) -> bool {
        let mut edited = String::with_capacity(source.len() + 1);
        edited.push_str(&source[..off]);
        edited.push(']');
        edited.push_str(&source[off..]);
        BracketIndex::build(&edited).unterminated_count() == 0
    }

    /// The production lexer's verdict: does `source` contain an
    /// unterminated `[` (a `missing close-bracket` warning, or a hard
    /// `LexError` for the same)?
    fn lexer_has_unterminated_bracket(source: &str) -> bool {
        match Lexer::new(source).tokenise_all_with_warnings() {
            Ok((_, warnings)) => warnings.iter().any(|w| w.message.contains("close-bracket")),
            // A hard error means the lexer bailed; treat a
            // close-bracket error as "unterminated", anything else as
            // inconclusive (excluded from the corpus by construction).
            Err(e) => format!("{e:?}").contains("close-bracket"),
        }
    }

    /// Deterministic LCG so the fuzz corpus is reproducible.
    struct Lcg(u64);
    impl Lcg {
        fn next_u32(&mut self) -> u32 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            (self.0 >> 33) as u32
        }
        fn pick<'a>(&mut self, xs: &[&'a str]) -> &'a str {
            xs[(self.next_u32() as usize) % xs.len()]
        }
    }

    /// Build a deterministic corpus of script-ish snippets from an
    /// alphabet that exercises every handled construct.
    fn corpus(n: usize) -> Vec<String> {
        // Tokens chosen to stress brackets across contexts: brace words,
        // quotes, escapes, `${}`, command subs, nested braces.
        let atoms = [
            "[", "]", "{", "}", "\"", "\\", "$", "a", " ", "\n", "x ", "${v}", "[a]", "{b}", "\\[",
            "\\]", "set ", "puts ",
        ];
        let mut out = Vec::with_capacity(n);
        let mut rng = Lcg(0x1234_5678_9abc_def0);
        for k in 0..n {
            let len = 1 + (rng.next_u32() as usize % 12);
            let mut s = String::new();
            for _ in 0..len {
                s.push_str(rng.pick(&atoms));
            }
            // Salt with the iteration so identical bodies still differ.
            if k % 7 == 0 {
                s.push('a');
            }
            out.push(s);
        }
        out
    }

    #[test]
    fn backslash_newline_continuation_opens_brace_word() {
        // RUST_ISSUE_088: a `{` on a `\<newline>`-continued line opens a brace
        // word, so the `[` inside `{[}` is inert — no unterminated verdict, and
        // the index must agree with the production lexer (the escape arm used to
        // clear `newword`, so `{[}` was scanned as bare text and the `[`
        // counted as an unterminated bracket).
        let src = "\\\n{[}";
        let idx_unterminated = BracketIndex::build(src).unterminated_count() > 0;
        assert!(
            !idx_unterminated,
            "brace word after a line continuation must not report an unterminated bracket",
        );
        assert_eq!(idx_unterminated, lexer_has_unterminated_bracket(src));
        // FP-guard: a genuine dangling `[` still counts.
        assert!(BracketIndex::build("a[").unterminated_count() > 0);
    }

    #[test]
    fn faithfulness_matches_production_lexer_unterminated_verdict() {
        // The index's unterminated-`[` verdict must equal the
        // production lexer's `missing close-bracket` warning across the
        // corpus — proving the scanner mirrors the lexer's bracket
        // rules (the doc's "store the lexer's entry-state" requirement).
        let mut total = 0usize;
        let mut agree = 0usize;
        let mut disagreements: Vec<String> = Vec::new();
        for s in corpus(8000) {
            let idx_unterminated = BracketIndex::build(&s).unterminated_count() > 0;
            let lex_unterminated = lexer_has_unterminated_bracket(&s);
            total += 1;
            if idx_unterminated == lex_unterminated {
                agree += 1;
            } else if disagreements.len() < 10 {
                disagreements.push(format!(
                    "{s:?}: index={idx_unterminated} lexer={lex_unterminated}"
                ));
            }
        }
        assert_eq!(
            agree,
            total,
            "{} / {total} disagreements with the production lexer; samples: {:#?}",
            total - agree,
            disagreements,
        );
    }

    /// Well-formed Tcl snippets the realistic-recovery corpus mutates by
    /// deleting one `]`. These mirror real code (the recovery target),
    /// not adversarial bracket soup.
    const WELL_FORMED: &[&str] = &[
        "puts [llength $x]\n",
        "set y [expr {[a] + [b]}]\n",
        "if {[string equal $a b]} { puts [foo] }\n",
        "set z [list a {b]c} d]\n",
        "proc p {} { return [bar [baz $q] $r] }\n",
        "foreach v [split $s ,] { incr n [expr {$v + 1}] }\n",
        "set d [dict create a [x] b [y]]\n",
        "puts \"value is [expr {$a * [b]}] done\"\n",
        "lappend out [string range $t 0 [expr {[string length $t] - 1}]]\n",
        "set r [regsub -all {\\d+} $line [string toupper [x]]]\n",
    ];

    #[test]
    fn adversarial_prediction_self_consistency_is_quantified() {
        // On adversarial random bracket/escape soup the index's
        // insert-here-balances prediction is a useful but NOT sound
        // approximation of its own full re-scan: inserting a `]` can
        // close a bracket early and re-contextualize the tail (a
        // count-based command-sub interior becomes top-level word-based),
        // split a `\\` pair and re-align escapes, or be consumed by a
        // following quote. The productionised forward-walk must re-derive
        // tail context after a hypothetical close. This tripwire
        // quantifies the boundary (and flags a regression if the
        // heuristic collapses) rather than asserting a soundness it does
        // not have. (The *realistic* recovery case — a single forgotten
        // `]` in real code — is verified exactly against C Tcl 9.0.3 in
        // `ctcl9_realistic_recovery_completes_under_reference`.)
        let mut total = 0usize;
        let mut agree = 0usize;
        for s in corpus(6000) {
            let idx = BracketIndex::build(&s);
            if idx.unterminated_count() == 0 {
                continue;
            }
            for off in 0..=s.len() {
                if !s.is_char_boundary(off) {
                    continue;
                }
                let pred = idx.close_bracket_balances(u32::try_from(off).unwrap());
                let truth = ground_truth_balances(&s, off);
                total += 1;
                if pred == truth {
                    agree += 1;
                }
            }
        }
        assert!(total > 1000, "corpus too small: {total}");
        // Integer >80% check (avoids float-cast lints).
        assert!(
            agree * 5 > total * 4,
            "adversarial agreement collapsed to {agree}/{total}",
        );
    }

    #[test]
    fn correction_extra_closer_closes_nothing() {
        // `]]]` with nothing open: the running level clamps at 0, so the
        // source is "balanced" (no unterminated `[`), matching the
        // lexer (extra `]` are literal, no warning).
        let idx = BracketIndex::build("]]]");
        assert_eq!(idx.unterminated_count(), 0);
        assert!(!lexer_has_unterminated_bracket("]]]"));
        // And `[][]` is balanced; `[[]` has one unterminated `[`.
        assert_eq!(BracketIndex::build("[][]").unterminated_count(), 0);
        assert_eq!(BracketIndex::build("[[]").unterminated_count(), 1);
    }

    #[test]
    fn correction_unterminated_opaque_to_eof() {
        // `[set x {a]b` — the `{` opens an unterminated brace word that
        // swallows to EOF. The `]` inside the brace word is literal, and
        // the `[` is unterminated.  Inserting `]` at EOF puts it inside
        // the unterminated brace word (still no `}`), so it is absorbed
        // as literal — it does NOT close the `[`.  This is the doc's
        // "unterminated opaque token reaching EOF makes the tail inert"
        // correction: a closer inserted after an unterminated span is
        // absorbed by *it*, not by the outer delimiter.
        let idx = BracketIndex::build("[set x {a]b");
        // The `]` inside the (unterminated) brace word is inert, so the
        // `[` is still unterminated.
        assert_eq!(idx.unterminated_count(), 1);
        // Inserting `]` at EOF is inert: absorbed by the unterminated brace.
        assert!(!idx.close_bracket_balances(u32::try_from("[set x {a]b".len()).unwrap()));
        // But inserting after the `[` (before the `{`) does close it.
        assert!(idx.close_bracket_balances(1));
        // Matches the production lexer.
        assert!(lexer_has_unterminated_bracket("[set x {a]b"));
    }

    #[test]
    fn scalar_only_index_diverges_from_lexer() {
        // A scalar level that ignores inert contexts (counts every `[` /
        // `]` literally) gets the wrong answer where the full index is
        // right — reproducing the doc's "a scalar level is not enough".
        fn scalar_unterminated(s: &str) -> i32 {
            let mut lvl = 0i32;
            for &b in s.as_bytes() {
                match b {
                    b'[' => lvl += 1,
                    b']' => lvl = (lvl - 1).max(0),
                    _ => {}
                }
            }
            lvl
        }
        // `puts {a]}` — the `]` is inside a brace word (literal); there
        // is no `[`, so the lexer sees no unterminated bracket.
        let s = "puts {a]}";
        assert!(!lexer_has_unterminated_bracket(s));
        assert_eq!(BracketIndex::build(s).unterminated_count(), 0); // full: correct
        assert_eq!(scalar_unterminated(s), 0); // (no `[` here either)

        // `set x {[}` — a `[` inside a brace word is literal; the lexer
        // sees NO unterminated bracket, but a scalar counter does.
        let s2 = "set x {[}";
        assert!(!lexer_has_unterminated_bracket(s2));
        assert_eq!(BracketIndex::build(s2).unterminated_count(), 0); // full: correct
        assert_eq!(scalar_unterminated(s2), 1); // scalar: WRONG (diverges)
    }

    // ---- C Tcl 9.0.3 differential verification (the reference
    // standard). The oracle is `tclsh9.0`'s `info complete`. These tests
    // skip gracefully when `tclsh9.0` is not on PATH so the suite still
    // passes in a minimal CI, but they run (and gate) wherever the
    // reference interpreter is available. ----

    /// Run `info complete` under C Tcl 9.0.3 for a batch of sources.
    /// Returns `None` if `tclsh9.0` is unavailable. Records are sent
    /// length-prefixed so arbitrary bytes (newlines, braces) round-trip.
    fn ctcl9_info_complete(sources: &[String]) -> Option<Vec<bool>> {
        use std::io::Write;
        use std::process::{Command, Stdio};

        const ORACLE: &str = r#"
fconfigure stdin -translation binary
fconfigure stdout -translation binary
while {1} {
    set line [gets stdin]
    if {[eof stdin] && $line eq ""} break
    if {$line eq ""} continue
    set nbytes [expr {int($line)}]
    set data [read stdin $nbytes]
    read stdin 1
    puts [info complete $data]
}
"#;
        let mut child = Command::new("tclsh9.0")
            .arg("-")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;
        {
            let mut stdin = child.stdin.take()?;
            stdin.write_all(ORACLE.as_bytes()).ok()?;
            for s in sources {
                let bytes = s.as_bytes();
                writeln!(stdin, "{}", bytes.len()).ok()?;
                stdin.write_all(bytes).ok()?;
                stdin.write_all(b"\n").ok()?;
            }
            // stdin dropped here -> EOF for the oracle loop.
        }
        let out = child.wait_with_output().ok()?;
        let text = String::from_utf8(out.stdout).ok()?;
        let verdicts: Vec<bool> = text.split_whitespace().map(|t| t == "1").collect();
        if verdicts.len() == sources.len() {
            Some(verdicts)
        } else {
            // Oracle desync (should not happen) — treat as unavailable
            // rather than asserting against a misaligned batch.
            None
        }
    }

    /// A corpus that isolates the **bracket** dimension for an
    /// `info complete` iff: braces and quotes appear only as *balanced,
    /// space-separated words* (`{b} `, `"q" `, `${v} `), so the only
    /// thing that can make a snippet incomplete is an unterminated `[`.
    /// Crucially every brace/quote word is followed by a separator, so
    /// the "extra characters after a close-brace/quote" class (which C
    /// Tcl 9.0.3 treats as *complete* — see
    /// `ctcl9_extra_chars_after_word_is_a_documented_divergence`) never
    /// arises.
    fn separated_corpus(n: usize) -> Vec<String> {
        // Each atom ends at a word boundary; no bare `{` `}` `"`.
        let atoms = [
            "a ",
            "x ",
            "set ",
            "puts ",
            "[a] ",
            "[x] ",
            "{b} ",
            "\"q\" ",
            "${v} ",
            "[ ",
            "] ",
            "\\[ ",
            "\\] ",
            "\n",
            "[set x ",
            "[expr 1] ",
        ];
        let mut out = Vec::with_capacity(n);
        let mut rng = Lcg(0xfeed_face_dead_beef);
        for _ in 0..n {
            let len = 1 + (rng.next_u32() as usize % 10);
            let mut s = String::new();
            for _ in 0..len {
                s.push_str(rng.pick(&atoms));
            }
            out.push(s);
        }
        out
    }

    #[test]
    fn ctcl9_bracket_completeness_iff_on_separated_corpus() {
        // **The reference-grounded bracket contract.** On a corpus where
        // braces / quotes are balanced and word-separated, C Tcl 9.0.3
        // `info complete` reflects *only* bracket balance, so the
        // structural index's verdict must match it exactly, both ways:
        //   index.unterminated_count()==0  <=>  info complete == 1.
        let cases = separated_corpus(8000);
        let Some(oracle) = ctcl9_info_complete(&cases) else {
            eprintln!("tclsh9.0 unavailable — skipping C Tcl 9.0.3 verification");
            return;
        };
        let mut both_complete = 0usize;
        let mut both_incomplete = 0usize;
        let mut violations: Vec<String> = Vec::new();
        for (s, &complete) in cases.iter().zip(&oracle) {
            let index_complete = BracketIndex::build(s).unterminated_count() == 0;
            if index_complete == complete {
                if complete {
                    both_complete += 1;
                } else {
                    both_incomplete += 1;
                }
            } else if violations.len() < 20 {
                violations.push(format!(
                    "{s:?}: index_complete={index_complete} tcl={complete}"
                ));
            }
        }
        assert!(
            violations.is_empty(),
            "index disagreed with C Tcl 9.0.3 `info complete`: {violations:#?}",
        );
        // The corpus must exercise both verdicts (not a degenerate pass).
        assert!(
            both_complete > 100 && both_incomplete > 100,
            "corpus not exercising both verdicts: complete={both_complete} incomplete={both_incomplete}",
        );
    }

    #[test]
    fn ctcl9_extra_chars_after_word_is_a_documented_divergence() {
        // **Finding (recorded in error-recovery-rust-port.md):** C Tcl
        // 9.0.3 treats a `[` (or `{` / `"`) *immediately* after a closing
        // brace / quote word, with no separator, as "extra characters
        // after close-brace" — a terminal parse error it reports as
        // `info complete == 1`. The lexer/index instead see the `[` as a
        // command substitution and (if unterminated) report an
        // unterminated bracket. This is a benign over-report for recovery
        // (we'd offer a `]` on an already-erroring line). Here we pin the
        // canonical example and prove the divergence is gone the moment a
        // separator is inserted.
        let Some(verdicts) = ctcl9_info_complete(&[
            "{b}[".to_string(),  // extra chars after `}` -> complete
            "a[".to_string(),    // command-sub in a bareword -> incomplete
            "{b} [".to_string(), // separated -> the `[` is a real, unterminated sub
        ]) else {
            eprintln!("tclsh9.0 unavailable — skipping C Tcl 9.0.3 verification");
            return;
        };
        assert_eq!(
            verdicts,
            vec![true, false, false],
            "C Tcl 9.0.3 semantics drifted"
        );
        // The index reports unterminated `[` for all three (it mirrors
        // the lexer, not the command parser's extra-chars rule).
        assert!(BracketIndex::build("{b}[").unterminated_count() > 0);
        assert!(BracketIndex::build("a[").unterminated_count() > 0);
        assert!(BracketIndex::build("{b} [").unterminated_count() > 0);
        // ...and once separated, index and reference agree (incomplete).
        assert_eq!(
            BracketIndex::build("{b} [").unterminated_count() > 0,
            !verdicts[2]
        );
    }

    #[test]
    fn ctcl9_realistic_recovery_completes_under_reference() {
        // For each well-formed snippet with one `]` deleted, the index
        // points at insert offsets that balance it; inserting `]` at the
        // *original* deleted position must yield a script C Tcl 9.0.3
        // calls complete, and the index must predict that. Direct
        // reference-grounded recovery check.
        let mut repaired: Vec<String> = Vec::new();
        let mut predicted: Vec<bool> = Vec::new();
        for base in WELL_FORMED {
            for (cp, _) in base.match_indices(']') {
                let mut broken = String::new();
                broken.push_str(&base[..cp]);
                broken.push_str(&base[cp + 1..]);
                let idx = BracketIndex::build(&broken);
                if idx.unterminated_count() == 0 {
                    continue;
                }
                // Re-insert `]` at the original site.
                let pred = idx.close_bracket_balances(u32::try_from(cp).unwrap());
                predicted.push(pred);
                repaired.push((*base).to_string()); // == broken with `]` back
            }
        }
        let Some(oracle) = ctcl9_info_complete(&repaired) else {
            eprintln!("tclsh9.0 unavailable — skipping C Tcl 9.0.3 verification");
            return;
        };
        for ((s, &complete), &pred) in repaired.iter().zip(&oracle).zip(&predicted) {
            assert!(complete, "expected reference-complete after repair: {s:?}");
            assert!(
                pred,
                "index failed to predict the original close site: {s:?}"
            );
        }
    }

    #[test]
    fn well_formed_command_subs_are_balanced() {
        for s in [
            "puts [llength $x]\n",
            "set y [expr {[a] + [b]}]\n",
            "if {[string equal $a b]} { puts hi }\n",
            "set z [list a {b]c} d]\n", // `]` inside nested brace is inert
        ] {
            assert_eq!(
                BracketIndex::build(s).unterminated_count(),
                0,
                "expected balanced: {s:?}",
            );
            assert!(!lexer_has_unterminated_bracket(s), "lexer parity: {s:?}");
        }
    }

    // Brace dimension (`{` / `}`) — same methodology as brackets.

    /// Top-level brace corpus (no `[…]` command subs), where the brace
    /// scanner's word-based + extra-chars + comment logic is faithful to
    /// `info complete`.
    fn brace_top_level_corpus(n: usize) -> Vec<String> {
        let atoms = [
            "{", "}", "\"", "\\", "$", "a", " ", "\n", "x ", "${v}", "{b}", "\\{", "\\}", "set ",
            "puts ", ";",
        ];
        let mut out = Vec::with_capacity(n);
        let mut rng = Lcg(0x5eed_1234_abcd_0001);
        for _ in 0..n {
            let len = 1 + (rng.next_u32() as usize % 12);
            let mut s = String::new();
            for _ in 0..len {
                s.push_str(rng.pick(&atoms));
            }
            out.push(s);
        }
        out
    }

    /// Full adversarial brace corpus *including* command substitutions
    /// and quotes — exercises the recursive `[...]`-interior parse.
    fn brace_full_corpus(n: usize) -> Vec<String> {
        let atoms = [
            "{", "}", "\"", "\\", "$", "a", " ", "\n", "x ", "${v}", "{b}", "[", "]", "[set x ",
            "[if {1} ", "\\{", "\\}", "set ", "puts ", "# ", ";", "{b}{",
        ];
        let mut out = Vec::with_capacity(n);
        let mut rng = Lcg(0x1dea_7777_3333_9999);
        for _ in 0..n {
            let len = 1 + (rng.next_u32() as usize % 11);
            let mut s = String::new();
            for _ in 0..len {
                s.push_str(rng.pick(&atoms));
            }
            out.push(s);
        }
        out
    }

    /// Brace-isolated corpus: brackets and quotes appear only as
    /// balanced, word-separated forms, and there are no comments or
    /// extra-`}`-after-close, so `info complete` reflects *only* brace
    /// balance.
    fn brace_separated_corpus(n: usize) -> Vec<String> {
        let atoms = [
            "a ",
            "x ",
            "set ",
            "puts ",
            "{b} ",
            "{a {b} c} ",
            "[x] ",
            "${v} ",
            "{ ",
            "set x { ",
            "\n",
            "foo {a ",
            "{} ",
        ];
        let mut out = Vec::with_capacity(n);
        let mut rng = Lcg(0xcafe_f00d_0042_1337);
        for _ in 0..n {
            let len = 1 + (rng.next_u32() as usize % 9);
            let mut s = String::new();
            for _ in 0..len {
                s.push_str(rng.pick(&atoms));
            }
            out.push(s);
        }
        out
    }

    #[test]
    fn ctcl9_brace_completeness_is_necessary_no_cmd_subs() {
        // Reference-grounded soundness across an adversarial *bracketless*
        // corpus (the brace scanner's top-level word-based + extra-chars +
        // comment logic is faithful to `info complete` here): whenever C
        // Tcl 9.0.3 reports the script complete, its braces must be
        // balanced, so the brace index must report zero unterminated `{`.
        //
        // Command substitutions are excluded on purpose: `info complete`
        // parses a `[…]` interior *recursively as a script* (word-based
        // braces + terminal extra-chars — `[set x {b}{` is **complete**),
        // whereas this prototype's `scan_cmd_sub` uses the lexer's
        // count-based rule. Faithful command-sub interiors need the full
        // recursive `Tcl_CommandComplete` parse — a documented boundary
        // (see the module header).
        let cases = brace_top_level_corpus(8000);
        let Some(oracle) = ctcl9_info_complete(&cases) else {
            eprintln!("tclsh9.0 unavailable — skipping C Tcl 9.0.3 verification");
            return;
        };
        let mut violations: Vec<String> = Vec::new();
        for (s, &complete) in cases.iter().zip(&oracle) {
            if complete && BraceIndex::build(s).unterminated_count() > 0 && violations.len() < 20 {
                violations.push(format!("{s:?}: tcl=complete but index=unterminated"));
            }
        }
        assert!(
            violations.is_empty(),
            "index found an unterminated `{{` C Tcl 9.0.3 considers complete: {violations:#?}",
        );
    }

    #[test]
    fn brace_command_sub_interiors_parse_as_scripts() {
        // Closed boundary: `[...]` interiors are parsed recursively as
        // scripts (word-based braces + terminal extra-chars), so the
        // index matches C Tcl 9.0.3 inside command subs.
        let pin: &[(&str, bool)] = &[
            ("[set x {b}{", true),        // extra-chars inside sub -> complete
            ("[set x {", false),          // genuinely open brace -> incomplete
            ("[set x {b}] {", false),     // open brace after a balanced sub
            ("[set x {b}{c}{", true),     // extra-chars after `{b}` -> complete
            ("x [set x {b}{", true),      // ditto, nested in a word
            ("[set x {a {b} c", false),   // nested unterminated brace word
            ("[if {1} {puts hi}]", true), // fully balanced
        ];
        for &(s, want_complete) in pin {
            assert_eq!(
                BraceIndex::build(s).unterminated_count() == 0,
                want_complete,
                "brace verdict for {s:?}",
            );
        }
        // Cross-check the pins against the reference interpreter.
        let inputs: Vec<String> = pin.iter().map(|(s, _)| (*s).to_string()).collect();
        if let Some(oracle) = ctcl9_info_complete(&inputs) {
            for ((s, want), &complete) in pin.iter().zip(&oracle) {
                assert_eq!(complete, *want, "C Tcl 9.0.3 semantics drifted for {s:?}");
            }
        }
    }

    #[test]
    fn ctcl9_brace_completeness_is_necessary_with_cmd_subs() {
        // With the recursive command-sub parse, the necessary condition
        // now holds across the *full* adversarial corpus (command subs
        // included): whenever C Tcl 9.0.3 reports complete, the brace
        // index must report balanced.
        let cases = brace_full_corpus(8000);
        let Some(oracle) = ctcl9_info_complete(&cases) else {
            eprintln!("tclsh9.0 unavailable — skipping C Tcl 9.0.3 verification");
            return;
        };
        let mut violations: Vec<String> = Vec::new();
        let mut complete_seen = 0usize;
        for (s, &complete) in cases.iter().zip(&oracle) {
            if complete {
                complete_seen += 1;
                if BraceIndex::build(s).unterminated_count() > 0 && violations.len() < 20 {
                    violations.push(format!("{s:?}: tcl=complete but index=unterminated"));
                }
            }
        }
        assert!(
            complete_seen > 100,
            "corpus exercised too few complete cases"
        );
        assert!(
            violations.is_empty(),
            "index found an unterminated `{{` C Tcl 9.0.3 considers complete: {violations:#?}",
        );
    }

    #[test]
    fn ctcl9_brace_completeness_iff_on_separated_corpus() {
        // Reference-grounded brace contract: on a corpus where brackets /
        // quotes are balanced and word-separated (and no comments /
        // extra-`}`), C Tcl 9.0.3 `info complete` reflects only brace
        // balance, so the brace index must match it both ways.
        let cases = brace_separated_corpus(8000);
        let Some(oracle) = ctcl9_info_complete(&cases) else {
            eprintln!("tclsh9.0 unavailable — skipping C Tcl 9.0.3 verification");
            return;
        };
        let mut both_complete = 0usize;
        let mut both_incomplete = 0usize;
        let mut violations: Vec<String> = Vec::new();
        for (s, &complete) in cases.iter().zip(&oracle) {
            let index_complete = BraceIndex::build(s).unterminated_count() == 0;
            if index_complete == complete {
                if complete {
                    both_complete += 1;
                } else {
                    both_incomplete += 1;
                }
            } else if violations.len() < 20 {
                violations.push(format!(
                    "{s:?}: index_complete={index_complete} tcl={complete}"
                ));
            }
        }
        assert!(
            violations.is_empty(),
            "brace index disagreed with C Tcl 9.0.3 `info complete`: {violations:#?}",
        );
        assert!(
            both_complete > 100 && both_incomplete > 100,
            "corpus not exercising both verdicts: complete={both_complete} incomplete={both_incomplete}",
        );
    }

    #[test]
    fn brace_semantics_match_reference_examples() {
        // Pin the canonical C Tcl 9.0.3 brace rules the index must honour.
        // (incomplete == unterminated_count > 0)
        let pin: &[(&str, bool)] = &[
            ("set x {", false),     // open brace word -> incomplete
            ("set x {a}", true),    // balanced
            ("a{", true),           // mid-word `{` is literal -> complete
            ("{", false),           // word-start open brace
            ("{a {b", false),       // depth 2 unterminated
            ("puts \"{\"", true),   // brace in quote is literal
            ("set x {a\\}", false), // `\}` escaped -> still open
            ("[set x {", false),    // brace in command sub counts
            ("set x {[}", true),    // `[` inside brace word is literal
            ("# {", true),          // brace in comment is ignored
            ("}{", true),           // extra `}` then mid-word `{` -> complete
            ("{}", true),           // balanced empty
        ];
        for &(s, want_complete) in pin {
            let complete = BraceIndex::build(s).unterminated_count() == 0;
            assert_eq!(complete, want_complete, "brace verdict for {s:?}");
        }
    }

    #[test]
    fn brace_correction_extra_closer_closes_nothing() {
        assert_eq!(BraceIndex::build("}}}").unterminated_count(), 0);
        assert_eq!(BraceIndex::build("{}{}").unterminated_count(), 0);
        assert_eq!(BraceIndex::build("{{}").unterminated_count(), 1);
    }

    #[test]
    fn brace_scalar_only_index_diverges_from_reference() {
        // A scalar counter that ignores word-position / quotes / comments
        // mis-judges `a{` (mid-word literal) and `# {` (comment).
        fn scalar_unterminated(s: &str) -> i32 {
            let mut lvl = 0i32;
            for &b in s.as_bytes() {
                match b {
                    b'{' => lvl += 1,
                    b'}' => lvl = (lvl - 1).max(0),
                    _ => {}
                }
            }
            lvl
        }
        for s in ["a{", "# {", "puts \"{\""] {
            assert_eq!(BraceIndex::build(s).unterminated_count(), 0, "full: {s:?}");
            assert!(scalar_unterminated(s) > 0, "scalar should diverge: {s:?}");
        }
    }

    #[test]
    fn ctcl9_realistic_brace_recovery_completes_under_reference() {
        // A single forgotten `}` in real code: the index predicts the
        // original close site and the repair is reference-complete.
        let bases = &[
            "proc p {} { return 1 }\n",
            "if {$x} { puts hi }\n",
            "namespace eval ns { variable v 1 }\n",
            "foreach a $l { incr n }\n",
            "set d {a b c}\n",
        ];
        let mut repaired: Vec<String> = Vec::new();
        let mut predicted: Vec<bool> = Vec::new();
        for base in bases {
            for (cp, _) in base.match_indices('}') {
                let mut broken = String::new();
                broken.push_str(&base[..cp]);
                broken.push_str(&base[cp + 1..]);
                let idx = BraceIndex::build(&broken);
                if idx.unterminated_count() == 0 {
                    continue;
                }
                predicted.push(idx.close_brace_balances(u32::try_from(cp).unwrap()));
                repaired.push((*base).to_string());
            }
        }
        let Some(oracle) = ctcl9_info_complete(&repaired) else {
            eprintln!("tclsh9.0 unavailable — skipping C Tcl 9.0.3 verification");
            return;
        };
        for ((s, &complete), &pred) in repaired.iter().zip(&oracle).zip(&predicted) {
            assert!(complete, "expected reference-complete after repair: {s:?}");
            assert!(
                pred,
                "index failed to predict the original close site: {s:?}"
            );
        }
    }

    // Expr paren dimension (`(` / `)`).

    /// C Tcl 9.0.3 `expr` paren verdict for a batch of expressions.
    /// `Some('B')` balanced / OK, `'O'` unbalanced-open, `'C'`
    /// unbalanced-close, `'X'` a non-paren error (paren state
    /// indeterminate from the oracle). Returns `None` if `tclsh9.0` is
    /// unavailable.
    fn tcl_expr_paren_verdicts(exprs: &[String]) -> Option<Vec<char>> {
        use std::io::Write;
        use std::process::{Command, Stdio};

        const ORACLE: &str = r#"
fconfigure stdin -translation binary
fconfigure stdout -translation binary
while {1} {
    set line [gets stdin]
    if {[eof stdin] && $line eq ""} break
    if {$line eq ""} continue
    set nbytes [expr {int($line)}]
    set data [read stdin $nbytes]
    read stdin 1
    if {[catch {expr $data} m]} {
        if {[string match {*unbalanced open paren*} $m]} {
            puts O
        } elseif {[string match {*unbalanced close paren*} $m]} {
            puts C
        } else {
            puts X
        }
    } else {
        puts B
    }
}
"#;
        let mut child = Command::new("tclsh9.0")
            .arg("-")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;
        {
            let mut stdin = child.stdin.take()?;
            stdin.write_all(ORACLE.as_bytes()).ok()?;
            for e in exprs {
                let bytes = e.as_bytes();
                writeln!(stdin, "{}", bytes.len()).ok()?;
                stdin.write_all(bytes).ok()?;
                stdin.write_all(b"\n").ok()?;
            }
        }
        let out = child.wait_with_output().ok()?;
        let text = String::from_utf8(out.stdout).ok()?;
        let verdicts: Vec<char> = text
            .split_whitespace()
            .filter_map(|t| t.chars().next())
            .collect();
        (verdicts.len() == exprs.len()).then_some(verdicts)
    }

    /// Adversarial expr corpus: parens, operands, operators, and the
    /// opaque tokens (`$arr(i)`, `[f]`, `"s"`, `{b}`) whose interior
    /// parens must not count.
    fn expr_corpus(n: usize) -> Vec<String> {
        let atoms = [
            "(", ")", "1", "2", " ", "+", "*", "+1", "*2", "$a", "$arr(1)", "[f]", "\"s)\"",
            "{x)}", "(1+2)", "a", "==", "!",
        ];
        let mut out = Vec::with_capacity(n);
        let mut rng = Lcg(0x00ff_a5a5_1234_9876);
        for _ in 0..n {
            let len = 1 + (rng.next_u32() as usize % 9);
            let mut s = String::new();
            for _ in 0..len {
                s.push_str(rng.pick(&atoms));
            }
            out.push(s);
        }
        out
    }

    #[test]
    fn expr_paren_index_counts_only_grouping_parens() {
        // `$arr(i)` / `"(s)"` / `{(b)}` / `[f(x)]` interiors do not count.
        assert_eq!(
            ExprParenIndex::build("$arr(1+2)").balance(),
            ParenBalance::Balanced
        );
        assert_eq!(
            ExprParenIndex::build("\"(((\"").balance(),
            ParenBalance::Balanced
        );
        assert_eq!(
            ExprParenIndex::build("{(((}").balance(),
            ParenBalance::Balanced
        );
        assert_eq!(
            ExprParenIndex::build("[f ((( ]").balance(),
            ParenBalance::Balanced
        );
        // Real grouping parens do count.
        assert_eq!(
            ExprParenIndex::build("(1+2)").balance(),
            ParenBalance::Balanced
        );
        assert_eq!(
            ExprParenIndex::build("(1+2").balance(),
            ParenBalance::OpenHeavy
        );
        assert_eq!(
            ExprParenIndex::build("1+2)").balance(),
            ParenBalance::CloseHeavy
        );
        assert_eq!(
            ExprParenIndex::build("$a(1)+(2").balance(),
            ParenBalance::OpenHeavy
        );
    }

    #[test]
    fn ctcl9_expr_paren_verdict_matches_reference() {
        // The index's paren-balance verdict must match C Tcl 9.0.3
        // `expr` wherever `expr` gives a *definitive* paren verdict (OK,
        // unbalanced-open, unbalanced-close). Non-paren errors ('X') are
        // excluded — `expr` may hit a different error before reaching the
        // paren check, leaving the paren state indeterminate from the
        // oracle.
        let cases = expr_corpus(8000);
        let Some(oracle) = tcl_expr_paren_verdicts(&cases) else {
            eprintln!("tclsh9.0 unavailable — skipping C Tcl 9.0.3 verification");
            return;
        };
        let mut checked = 0usize;
        let mut violations: Vec<String> = Vec::new();
        for (s, &v) in cases.iter().zip(&oracle) {
            let want = match v {
                'B' => ParenBalance::Balanced,
                'O' => ParenBalance::OpenHeavy,
                'C' => ParenBalance::CloseHeavy,
                _ => continue, // 'X' indeterminate
            };
            checked += 1;
            let got = ExprParenIndex::build(s).balance();
            if got != want && violations.len() < 20 {
                violations.push(format!("{s:?}: index={got:?} tcl={want:?}"));
            }
        }
        assert!(checked > 1000, "too few definitive cases: {checked}");
        assert!(
            violations.is_empty(),
            "expr paren index disagreed with C Tcl 9.0.3: {violations:#?}",
        );
    }

    #[test]
    fn ctcl9_realistic_expr_paren_recovery() {
        // A single forgotten `)` in a real expression: the index
        // predicts the original close site, and the repair evaluates
        // under C Tcl 9.0.3.
        let bases = &[
            "(1 + 2) * 3",
            "max(1, (2 + 3))",
            "($a + $b) / ($c - 1)",
            "(($x == 1) && ($y != 2))",
            "abs((1 - 2) * (3 + 4))",
        ];
        let mut repaired: Vec<String> = Vec::new();
        let mut predicted: Vec<bool> = Vec::new();
        for base in bases {
            for (cp, _) in base.match_indices(')') {
                let mut broken = String::new();
                broken.push_str(&base[..cp]);
                broken.push_str(&base[cp + 1..]);
                let idx = ExprParenIndex::build(&broken);
                if idx.balance() != ParenBalance::OpenHeavy {
                    continue;
                }
                predicted.push(idx.close_paren_balances(u32::try_from(cp).unwrap()));
                repaired.push((*base).to_string());
            }
        }
        let Some(oracle) = tcl_expr_paren_verdicts(&repaired) else {
            eprintln!("tclsh9.0 unavailable — skipping C Tcl 9.0.3 verification");
            return;
        };
        for ((s, &v), &pred) in repaired.iter().zip(&oracle).zip(&predicted) {
            assert_ne!(v, 'O', "repaired expr still open-heavy: {s:?}");
            assert_ne!(v, 'C', "repaired expr close-heavy: {s:?}");
            assert!(
                pred,
                "index failed to predict the original close site: {s:?}"
            );
        }
    }

    #[test]
    fn expr_paren_correction_extra_closer_clamps() {
        // Extra `)` clamps in `unmatched_opens` (recovery never inserts a
        // `)` to fix a close-heavy expr), but `balance` still reports the
        // close-heavy direction.
        assert_eq!(ExprParenIndex::build(")))").unmatched_opens(), 0);
        assert_eq!(
            ExprParenIndex::build(")))").balance(),
            ParenBalance::CloseHeavy
        );
        assert_eq!(ExprParenIndex::build("()()").unmatched_opens(), 0);
        assert_eq!(ExprParenIndex::build("(((").unmatched_opens(), 3);
    }

    #[test]
    fn expr_paren_scalar_diverges_on_opaque_tokens() {
        fn scalar(s: &str) -> bool {
            let mut lvl = 0i32;
            for &b in s.as_bytes() {
                match b {
                    b'(' => lvl += 1,
                    b')' => lvl -= 1,
                    _ => {}
                }
            }
            lvl == 0
        }
        // `$arr(1)` and `"(s)"`: balanced for the index (opaque), but a
        // scalar counter that ignores opacity also happens to balance —
        // use an *unbalanced* interior to force divergence.
        for s in ["$arr((1)", "\"(((\"", "{(((}"] {
            assert_eq!(
                ExprParenIndex::build(s).balance(),
                ParenBalance::Balanced,
                "index should see balanced (opaque): {s:?}",
            );
            assert!(!scalar(s), "scalar counter should diverge: {s:?}");
        }
    }

    // Unified script completeness (`script_is_complete` vs `info complete`).

    #[test]
    fn script_is_complete_matches_reference_pins() {
        // Canonical `info complete` cases spanning every dimension.
        let pin: &[(&str, bool)] = &[
            ("puts hi", true),
            ("puts [llength $x]", true),
            ("puts [llength $x", false), // open bracket
            ("set x {a", false),         // open brace
            ("set x {a}", true),
            ("a{", true),         // mid-word `{` literal
            ("puts \"hi", false), // open quote
            ("puts \"hi\"", true),
            ("{b}[", true),        // extra-chars after brace word
            ("[set x {b}{", true), // extra-chars inside cmd sub
            ("[set x {", false),   // open brace in cmd sub
            ("# {", true),         // comment
            (" # {", true),        // comment after leading ws
            ("a\\\n", false),      // trailing line continuation
            ("a\\", true),         // bare trailing backslash
            ("a\\\\\n", true),     // escaped backslash, then newline
            ("set x ${a}", true),
            ("set x ${a", false),  // unterminated `${`
            ("if {1} {\n", false), // open body brace
            ("proc p {} { return 1 }\n", true),
            ("}{", true), // extra close then mid-word `{`
            // Extracted-helper coverage (oracle: tclsh9.0 `info complete`).
            // step_brace_word — nested braces inside a brace word.
            ("set x {a {b} c}", true), // balanced nested brace word
            ("set x {a {b c}", false), // nested brace word still open
            ("set x ${a{b}c}", true),  // `${...}` var brace with nested `{}`
            ("${a}b", true),           // var-brace continues the word on close
            ("{a}b", true),            // arg-brace close + extra chars (terminal)
            ("set x {a \\} b}", true), // `\}` is escaped, does not close
            ("set x {a \\} b", false), // ...so the word is still open at EOF
            // skip_comment — backslash-newline continues the comment onto a
            // line that carries content (not the documented EOF edge).
            ("# c\\\nputs hi", true),
        ];
        for &(s, want) in pin {
            assert_eq!(script_is_complete(s), want, "script_is_complete({s:?})");
        }
    }

    /// General adversarial corpus mixing every construct — used for the
    /// direct `info complete` iff (no dimension isolation needed).
    fn general_corpus(n: usize) -> Vec<String> {
        let atoms = [
            "{", "}", "[", "]", "\"", "\\", "$", "a", " ", "\n", ";", "x ", "set ", "puts ",
            "${v}", "{b}", "[a]", "[set x ", "if {1} ", "# c", "\\\n", "\\\\", "${", "1+1",
        ];
        let mut out = Vec::with_capacity(n);
        let mut rng = Lcg(0xabcd_1234_5678_9f0e);
        for _ in 0..n {
            let len = 1 + (rng.next_u32() as usize % 12);
            let mut s = String::new();
            for _ in 0..len {
                s.push_str(rng.pick(&atoms));
            }
            out.push(s);
        }
        out
    }

    #[test]
    fn ctcl9_script_is_complete_iff_info_complete() {
        // The full contract: `script_is_complete` matches C Tcl 9.0.3
        // `info complete` *both ways* over a general adversarial corpus —
        // no dimension isolation, every construct mixed.
        let cases = general_corpus(12000);
        let Some(oracle) = ctcl9_info_complete(&cases) else {
            eprintln!("tclsh9.0 unavailable — skipping C Tcl 9.0.3 verification");
            return;
        };
        let mut both_t = 0usize;
        let mut both_f = 0usize;
        let mut disagreements = 0usize;
        let mut unexpected: Vec<String> = Vec::new();
        for (s, &complete) in cases.iter().zip(&oracle) {
            let got = script_is_complete(s);
            if got == complete {
                if complete {
                    both_t += 1;
                } else {
                    both_f += 1;
                }
                continue;
            }
            disagreements += 1;
            // Every disagreement must be the documented comment
            // backslash-newline-continuation-at-EOF edge: the source has
            // a `#` comment and a backslash-newline. Anything else is a
            // real regression.
            let is_comment_bs_edge = s.contains('#') && s.contains("\\\n");
            if !is_comment_bs_edge && unexpected.len() < 25 {
                unexpected.push(format!("{s:?}: ours={got} tcl={complete}"));
            }
        }
        assert!(
            unexpected.is_empty(),
            "script_is_complete disagreed with C Tcl 9.0.3 outside the documented \
             comment-continuation edge: {unexpected:#?}",
        );
        // The documented edge must stay vanishingly rare (< 0.05%).
        assert!(
            disagreements * 2000 < cases.len(),
            "too many disagreements ({disagreements}/{}) — the comment edge regressed",
            cases.len(),
        );
        assert!(
            both_t > 200 && both_f > 200,
            "corpus not exercising both verdicts: complete={both_t} incomplete={both_f}",
        );
    }

    // Incremental-reparse primitives.

    #[test]
    fn command_boundaries_split_top_level_commands() {
        assert_eq!(command_boundaries("set x 1\nputs hi\n"), vec![8, 16]);
        assert_eq!(command_boundaries("a; b; c"), vec![2, 5, 7]);
        // No trailing terminator: the final boundary is the length.
        assert_eq!(command_boundaries("set x 1"), vec![7]);
        assert_eq!(command_boundaries(""), vec![0]);
    }

    #[test]
    fn command_boundaries_ignore_nested_separators() {
        // Newlines inside a brace body / quoted string / command sub do
        // not split a top-level command.
        let src = "if {1} {\n  puts a\n  puts b\n}\nputs done\n";
        let bounds = command_boundaries(src);
        // Two commands: the whole `if {...}` then `puts done`.
        assert_eq!(bounds.len(), 2, "{bounds:?}");
        // The first boundary is just past the `}` that closes the body.
        let close = src.find("}\n").unwrap() + 2;
        assert_eq!(bounds[0] as usize, close);

        // `;` inside a quoted string and a command sub don't split.
        let q = "puts \"a;b\"\nset y [a;b]\n";
        assert_eq!(command_boundaries(q), vec![11, 23]);
    }

    #[test]
    fn command_boundaries_are_complete_prefixes() {
        // Every boundary is a point where the prefix is a complete script
        // — the invariant that makes it a safe incremental split point.
        for s in general_corpus(6000) {
            for &b in &command_boundaries(&s) {
                if (b as usize) < s.len() && s.is_char_boundary(b as usize) {
                    assert!(
                        script_is_complete(&s[..b as usize]),
                        "boundary {b} of {s:?} is not a complete prefix",
                    );
                }
            }
        }
    }

    #[test]
    fn reparse_window_snaps_to_whole_commands() {
        let src = "set x 1\nputs hi\nset y 2\n"; // boundaries: 8, 16, 24
        // An edit inside `puts hi` (bytes 8..16) snaps to exactly that
        // command's window.
        assert_eq!(reparse_window(src, 10, 12), (8, 16));
        // An edit spanning the first two commands snaps to cover both.
        assert_eq!(reparse_window(src, 3, 10), (0, 16));
        // An edit at the very start.
        assert_eq!(reparse_window(src, 0, 1), (0, 8));
        // A point edit at a boundary stays minimal.
        assert_eq!(reparse_window(src, 16, 16), (16, 16));
    }

    #[test]
    fn reparse_window_is_aligned_and_contains_edit() {
        // Property check over a corpus: the window is boundary-aligned,
        // contains the edit, and is minimal (no boundary strictly inside
        // (lo, start] or [end, hi)).
        let mut rng = Lcg(0x7777_3333_1111_9999);
        for s in general_corpus(4000) {
            if s.is_empty() {
                continue;
            }
            let len = u32::try_from(s.len()).unwrap();
            let a = rng.next_u32() % (len + 1);
            let b = a + rng.next_u32() % (len + 1 - a);
            let (lo, hi) = reparse_window(&s, a, b);
            assert!(
                lo <= a && b <= hi && hi <= len,
                "window ({lo},{hi}) edit ({a},{b}) len {len}"
            );
            let bounds = command_boundaries(&s);
            let aligned = |x: u32| x == 0 || x == len || bounds.contains(&x);
            assert!(
                aligned(lo) && aligned(hi),
                "unaligned window ({lo},{hi}) for {s:?}"
            );
            // Minimality: no boundary in (lo, a] (else lo wasn't maximal).
            assert!(
                !bounds.iter().any(|&x| x > lo && x <= a),
                "non-minimal lo for {s:?}",
            );
        }
    }
}
