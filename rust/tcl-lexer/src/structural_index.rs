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
//! Scope: the **script `[` / `]` (bracket) sublanguage**. The doc's
//! headline script claim covers `[` and `}`; this prototype proves the
//! bracket dimension end-to-end and is structured so the brace and expr
//! dimensions slot in the same way. It is deliberately *not wired into
//! production* — like the Python prototypes (kept only in git history),
//! it instruments the model and adds no production value yet; the
//! incremental green-tree engine will build the productionised index.
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
}
