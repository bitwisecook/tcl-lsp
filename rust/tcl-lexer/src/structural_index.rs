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
//! production* — like the Python prototypes (kept only in git history),
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
//! ### Brace-dimension boundary (documented, pinned)
//!
//! `info complete` parses a `[…]` **interior recursively as a script**
//! (word-based braces + terminal "extra characters after close-brace"),
//! so `[set x {b}{` is **complete** even though the outer `[` is
//! unterminated. The prototype's `scan_cmd_sub` uses the lexer's
//! *count-based* brace rule and so over-reports unterminated braces
//! *inside* command substitutions. Faithful command-sub interiors need
//! the full recursive `Tcl_CommandComplete` parse. The brace index is
//! verified against C Tcl 9.0.3 on the bracketless and bracket-isolated
//! corpora where it is faithful, and the boundary is pinned by
//! `ctcl9_command_sub_interior_is_a_documented_boundary`.
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
    /// Inert byte ranges `[start, end)` where `[` / `]` are literal
    /// (brace words, `${…}`, escape pairs, command-sub `{…}` interiors).
    /// Sorted, non-overlapping.
    inert: Vec<(u32, u32)>,
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
        let mut merged: Vec<(u32, u32)> = Vec::with_capacity(b.inert.len());
        for (s, e) in b.inert {
            match merged.last_mut() {
                Some(last) if s <= last.1 => last.1 = last.1.max(e),
                _ => merged.push((s, e)),
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
        let idx = self.inert.partition_point(|&(s, _)| s <= off);
        if idx == 0 {
            return false;
        }
        let (_, end) = self.inert[idx - 1];
        off < end
    }

    /// `true` when inserting a closer *at* `off` would be absorbed as a
    /// literal — i.e. `off` is **strictly inside** an inert span (`start
    /// < off < end`). Insertion at a span's start sits *before* the
    /// inert run (e.g. before a `\` or a `{`), so the closer is
    /// structural there; only insertion between the bytes is absorbed
    /// (e.g. `\` + inserted `]` -> `\]`).
    #[must_use]
    fn inert_for_insert(&self, off: u32) -> bool {
        let idx = self.inert.partition_point(|&(s, _)| s < off);
        if idx == 0 {
            return false;
        }
        let (start, end) = self.inert[idx - 1];
        start < off && off < end
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
    inert: Vec<(u32, u32)>,
}

impl Builder<'_> {
    fn push_inert(&mut self, start: usize, end: usize) {
        if end > start {
            self.inert.push((
                u32::try_from(start).expect("offset fits u32"),
                u32::try_from(end).expect("offset fits u32"),
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
                    // brackets.
                    let end = (i + 2).min(n);
                    self.push_inert(i, end);
                    i = end;
                    newword = false;
                }
                b' ' | b'\t' | b'\r' | b'\n' | b';' if !in_quote => {
                    i += 1;
                    newword = true;
                }
                b'{' if newword && !in_quote => {
                    // Verbatim brace word — the whole `{…}` span is
                    // inert for brackets.  STR keeps `newword` true.
                    let end = scan_brace_word(self.bytes, i);
                    self.push_inert(i, end);
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
                    let end = scan_dollar_brace(self.bytes, i);
                    self.push_inert(i, end);
                    i = end;
                    newword = false;
                }
                b'[' => {
                    self.push_event(i, 1);
                    i = self.scan_cmd_sub(i + 1);
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
    /// (just past the `[`). Mirrors `Lexer::scan_command_substitution`'s
    /// **count-based** rules: `blevel` tracks `{` / `}` literally, a `]`
    /// closes only at `blevel == 0 && !in_quotes`, nested `[` / `]`
    /// recurse. Records nested structural bracket events and inert spans
    /// for brace interiors / escapes / `${…}`. Returns the offset just
    /// past the closing `]`, or EOF if unterminated.
    fn scan_cmd_sub(&mut self, start: usize) -> usize {
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
                    i = self.scan_cmd_sub(i + 1);
                }
                b']' if blevel == 0 && !in_quotes => {
                    self.push_event(i, -1);
                    return i + 1;
                }
                b'\\' => {
                    let end = (i + 2).min(n);
                    self.push_inert(i, end);
                    i = end;
                }
                b'$' if !in_quotes && blevel == 0 && self.bytes.get(i + 1) == Some(&b'{') => {
                    let end = scan_dollar_brace(self.bytes, i);
                    self.push_inert(i, end);
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
                    if blevel == 0 {
                        if let Some(s) = brace_run_start.take() {
                            // Mark the closed `{…}` run inert for
                            // brackets (a `]` inside it never counted).
                            self.push_inert(s, i);
                        }
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
            self.push_inert(s, n);
        }
        n
    }
}

/// Scan a verbatim brace word starting at the opening `{` at `start`.
/// Counts nested `{` / `}` (a `\}` does not close — the pair is
/// consumed). Returns the offset just past the matching `}`, or EOF if
/// unterminated. Mirrors `Lexer::parse_brace`.
fn scan_brace_word(bytes: &[u8], start: usize) -> usize {
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
                    return i;
                }
            }
            _ => i += 1,
        }
    }
    n
}

/// Scan a `${…}` variable-name brace starting at the `$` at `start`.
/// Returns the offset just past the matching `}`, or EOF if
/// unterminated. The interior is inert for brackets.
fn scan_dollar_brace(bytes: &[u8], start: usize) -> usize {
    let n = bytes.len();
    let mut i = start + 2; // skip `${`
    while i < n {
        if bytes[i] == b'}' {
            return i + 1;
        }
        i += 1;
    }
    n
}

// ===========================================================================
// Brace dimension (`{` / `}`) — the second script sublanguage the doc names
// alongside brackets. Same methodology and decision procedure; the *rules*
// differ: a `{` opens a brace word only at a word boundary (mid-word `{` is
// literal — `a{` is complete in C Tcl 9.0.3), inside a brace word nesting is
// verbatim (`\}` does not close), and a `#` at command start begins a comment
// whose braces are ignored. Quotes make braces literal; command-substitution
// interiors count braces (`[set x {` is incomplete).
// ===========================================================================

/// A structural-state index for the script brace (`{` / `}`) dimension,
/// captured in a single forward scan. See the module header; the decision
/// procedure mirrors [`BracketIndex`].
#[derive(Debug, Clone)]
pub struct BraceIndex {
    /// Structural brace events in ascending offset order: `+1` for a
    /// structural `{`, `-1` for a structural `}`.
    events: Vec<(u32, i32)>,
    /// Inert byte ranges `[start, end)` where `{` / `}` are literal
    /// (comments, quoted runs, escape pairs, `${…}`). Sorted, merged.
    inert: Vec<(u32, u32)>,
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
        let mut merged: Vec<(u32, u32)> = Vec::with_capacity(b.inert.len());
        for (s, e) in b.inert {
            match merged.last_mut() {
                Some(last) if s <= last.1 => last.1 = last.1.max(e),
                _ => merged.push((s, e)),
            }
        }
        BraceIndex {
            events: b.events,
            inert: merged,
            len: u32::try_from(bytes.len()).expect("source length fits u32"),
        }
    }

    /// `true` when inserting a `}` *at* `off` would be absorbed as a
    /// literal (strictly inside an inert span).
    #[must_use]
    fn inert_for_insert(&self, off: u32) -> bool {
        let idx = self.inert.partition_point(|&(s, _)| s < off);
        if idx == 0 {
            return false;
        }
        let (start, end) = self.inert[idx - 1];
        start < off && off < end
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
    inert: Vec<(u32, u32)>,
}

impl BraceBuilder<'_> {
    fn push_inert(&mut self, start: usize, end: usize) {
        if end > start {
            self.inert.push((
                u32::try_from(start).expect("offset fits u32"),
                u32::try_from(end).expect("offset fits u32"),
            ));
        }
    }

    fn push_event(&mut self, off: usize, delta: i32) {
        self.events
            .push((u32::try_from(off).expect("offset fits u32"), delta));
    }

    /// Top-level command/word context scan for the brace dimension.
    #[allow(clippy::too_many_lines)] // one cohesive state machine
    fn scan_top(&mut self) {
        let n = self.bytes.len();
        let mut i = 0usize;
        // `command_start`: a `#` here begins a comment (only at the
        // start of a command — after Eol / `;` / source start).
        let mut command_start = true;
        // `is_newword`: a `{` / `"` here is a delimiter-word opener.
        let mut newword = true;
        // Verbatim brace-word nesting depth (0 = command/word context).
        let mut brace_level: u32 = 0;
        // Set right after a brace/quote *word* closes: the next char
        // decides whether it is a separator (normal) or "extra
        // characters after close-brace/quote" — a terminal parse error C
        // Tcl 9.0.3 reports as *complete* (`{b}{`, `"x"{`, `{a}}` are all
        // complete). On that error we stop the scan: nothing after it
        // affects completeness.
        let mut just_closed_word = false;
        while i < n {
            if brace_level > 0 {
                // Verbatim brace-word context: nested `{` / `}` count,
                // `\}` is escaped (does not close).
                match self.bytes[i] {
                    b'\\' => {
                        let end = (i + 2).min(n);
                        self.push_inert(i, end);
                        i = end;
                    }
                    b'{' => {
                        brace_level += 1;
                        self.push_event(i, 1);
                        i += 1;
                    }
                    b'}' => {
                        brace_level -= 1;
                        self.push_event(i, -1);
                        i += 1;
                        if brace_level == 0 {
                            newword = true;
                            command_start = false;
                            just_closed_word = true;
                        }
                    }
                    _ => i += 1,
                }
                continue;
            }
            // "Extra characters after close-brace/quote" gate.
            if just_closed_word {
                match self.bytes[i] {
                    b' ' | b'\t' | b'\r' | b'\n' | b';' => just_closed_word = false,
                    // A backslash-newline continuation after a close is
                    // fine (the lexer special-cases it); anything else is
                    // the terminal extra-chars error -> stop.
                    b'\\' if matches!(self.bytes.get(i + 1), Some(b'\n' | b'\r')) => {
                        just_closed_word = false;
                    }
                    _ => return,
                }
            }
            // Command / word context.
            match self.bytes[i] {
                b'#' if command_start => {
                    // Comment to end of line — braces inside are inert.
                    let mut j = i;
                    while j < n && self.bytes[j] != b'\n' {
                        j += 1;
                    }
                    self.push_inert(i, j);
                    i = j;
                }
                b'\\' => {
                    let end = (i + 2).min(n);
                    self.push_inert(i, end);
                    i = end;
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
                    command_start = false;
                }
                b'{' if newword => {
                    brace_level = 1;
                    self.push_event(i, 1);
                    i += 1;
                    command_start = false;
                }
                b'"' if newword => {
                    i = self.scan_quoted(i + 1);
                    newword = false;
                    command_start = false;
                    just_closed_word = true;
                }
                b'$' if self.bytes.get(i + 1) == Some(&b'{') => {
                    // `${name}` is a substitution, not a brace word: a
                    // balanced one is inert, but an *unterminated* `${`
                    // is a missing-close-brace (`${a` is incomplete).
                    let end = scan_dollar_brace(self.bytes, i);
                    if end >= n && self.bytes.get(n - 1) != Some(&b'}') {
                        self.push_event(i + 1, 1); // unterminated `{`
                    }
                    self.push_inert(i, end);
                    i = end;
                    newword = false;
                    command_start = false;
                }
                b'[' => {
                    i = self.scan_cmd_sub(i + 1);
                    newword = false;
                    command_start = false;
                }
                b'}' => {
                    // A `}` with no open brace word: extra/literal close;
                    // record it so `unterminated_count` clamps (matching
                    // C Tcl, where `}{` is complete).
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

    /// Scan a `"…"` quoted run (the opening `"` already consumed).
    /// Braces inside are literal, but command substitutions inside still
    /// count braces. Returns the offset past the closing `"`, or EOF.
    fn scan_quoted(&mut self, start: usize) -> usize {
        let n = self.bytes.len();
        let mut i = start;
        while i < n {
            match self.bytes[i] {
                b'\\' => i = (i + 2).min(n),
                b'"' => return i + 1,
                b'[' => i = self.scan_cmd_sub(i + 1),
                b'$' if self.bytes.get(i + 1) == Some(&b'{') => {
                    i = scan_dollar_brace(self.bytes, i);
                }
                _ => i += 1,
            }
        }
        n
    }

    /// Scan a `[…]` command-substitution interior (the `[` already
    /// consumed), counting braces with the command-sub's count-based
    /// rules (mirrors `Lexer::scan_command_substitution`). Returns the
    /// offset past the closing `]`, or EOF.
    fn scan_cmd_sub(&mut self, start: usize) -> usize {
        let n = self.bytes.len();
        let mut i = start;
        let mut in_quotes = false;
        let mut blevel: u32 = 0;
        while i < n {
            match self.bytes[i] {
                b'"' if blevel == 0 => {
                    in_quotes = !in_quotes;
                    i += 1;
                }
                b'[' if blevel == 0 && !in_quotes => {
                    i = self.scan_cmd_sub(i + 1);
                }
                b']' if blevel == 0 && !in_quotes => {
                    return i + 1;
                }
                b'\\' => i = (i + 2).min(n),
                b'$' if !in_quotes && blevel == 0 && self.bytes.get(i + 1) == Some(&b'{') => {
                    let end = scan_dollar_brace(self.bytes, i);
                    self.push_inert(i, end);
                    i = end;
                }
                b'{' if !in_quotes => {
                    blevel += 1;
                    self.push_event(i, 1);
                    i += 1;
                }
                b'}' if !in_quotes => {
                    blevel = blevel.saturating_sub(1);
                    self.push_event(i, -1);
                    i += 1;
                }
                _ => i += 1,
            }
        }
        n
    }
}

// ===========================================================================
// Expr paren dimension (`(` / `)`) — the doc's third structural index. Parens
// nest in expressions where they don't at script level, and the opaque tokens
// `[…]` / `"…"` / `{…}` / `${…}` / `$arr(idx)` are whole tokens whose interior
// parens never count. The Rust expr lexer already tokenises exactly that way
// (`$arr(idx)` is one `Variable`; strings / command subs are whole tokens), so
// the paren index is built **directly from the lexer's token stream** — the
// most faithful possible "store the lexer's entry-state per token".
// ===========================================================================

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
        // `set x {a` — an unterminated brace word swallows to EOF; the
        // `[` that *precedes* nothing here is absent, but a `[` before
        // an unterminated brace stays the outermost open and an inserted
        // `]` closes it (the EOF-inert correction in action).
        let idx = BracketIndex::build("[set x {a]b");
        // The `]` inside the (unterminated) brace word is inert, so the
        // `[` is still unterminated.
        assert_eq!(idx.unterminated_count(), 1);
        // Inserting `]` at EOF closes the outer `[`.
        assert!(idx.close_bracket_balances(u32::try_from("[set x {a]b".len()).unwrap()));
        // Parity with the production lexer.
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

    // ===================================================================
    // Brace dimension (`{` / `}`) — same methodology as brackets.
    // ===================================================================

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
    fn ctcl9_command_sub_interior_is_a_documented_boundary() {
        // Pin the command-sub-interior recursion boundary: C Tcl 9.0.3
        // parses `[…]` interiors as scripts, so a brace word followed by
        // a non-separator inside a `[…]` is a *terminal* extra-chars error
        // -> complete, even though the outer `[` is unterminated. The
        // count-based `scan_cmd_sub` does not model that recursion, so it
        // over-reports here.
        let Some(v) = ctcl9_info_complete(&[
            "[set x {b}{".to_string(),   // extra-chars inside cmd sub -> complete
            "[set x {".to_string(),      // genuinely open brace -> incomplete
            "[set x {b}] {".to_string(), // top-level open brace after balanced sub
        ]) else {
            eprintln!("tclsh9.0 unavailable — skipping C Tcl 9.0.3 verification");
            return;
        };
        assert_eq!(v, vec![true, false, false], "C Tcl 9.0.3 semantics drifted");
        // Top-level cases the prototype *does* get right:
        assert!(BraceIndex::build("[set x {").unterminated_count() > 0);
        assert!(BraceIndex::build("[set x {b}] {").unterminated_count() > 0);
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

    // ===================================================================
    // Expr paren dimension (`(` / `)`).
    // ===================================================================

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
}
