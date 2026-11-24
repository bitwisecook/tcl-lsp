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
    // A `{` on a `\<newline>`-continued line opens a brace
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
    // close a bracket early and re-contextualise the tail (a
    // count-based command-sub interior becomes top-level word-based),
    // split a `\\` pair and re-align escapes, or be consumed by a
    // following quote. The productionised forward-walk must re-derive
    // tail context after a hypothetical close. This tripwire
    // quantifies the boundary (and flags a regression if the
    // heuristic collapses) rather than asserting a soundness it does
    // not have. (The *realistic* recovery case — a single forgotten
    // `]` in real code — is verified exactly against C Tcl 9.0 in
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
fn bracket_index_comment_hides_command_closer() {
    // A command-position comment is part of the nested script.  Its `]`
    // must be inert, leaving only the final `]` as the substitution's
    // structural closer.
    let src = "[\n# ] hidden\nset y 1\n]";
    let close = u32::try_from(src.len() - 1).unwrap();
    assert_eq!(BracketIndex::build(src).events(), &[(0, 1), (close, -1)]);

    // Horizontal whitespace separates words but does not start a new
    // command: this hash is ordinary word content, so the `]` remains a
    // structural extra closer at top level.
    let ordinary_hash = "set x #not-comment]";
    assert!(
        BracketIndex::build(ordinary_hash)
            .events()
            .iter()
            .any(|&(off, delta)| {
                off == u32::try_from(ordinary_hash.len() - 1).unwrap() && delta == -1
            })
    );
    let var_then_hash = "[${x} #not-comment]";
    assert!(
        BracketIndex::build(var_then_hash)
            .events()
            .iter()
            .any(|&(off, delta)| {
                off == u32::try_from(var_then_hash.len() - 1).unwrap() && delta == -1
            })
    );

    // Raw CRLF is not a continuation at the parser seam. The escaped CR
    // ends before the LF, so the first bracket on the next line closes
    // the substitution and the final bracket is an extra closer.
    let crlf = "[\r\n# hidden \\\r\n] still hidden\r\nset y 1]";
    assert_eq!(
        BracketIndex::build(crlf).events(),
        &[
            (0, 1),
            (u32::try_from(crlf.find(']').unwrap()).unwrap(), -1),
            (u32::try_from(crlf.len() - 1).unwrap(), -1),
        ]
    );
    let even = "[\n# two \\\\\n]";
    let events = BracketIndex::build(even);
    assert_eq!(events.events().iter().filter(|(_, d)| *d == -1).count(), 1);
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

// C Tcl 9.0 differential verification. The oracle is `tclsh9.0`'s
// `info complete`. These tests
// skip gracefully when the Tcl 9.0 oracle is unavailable so the suite still
// passes in a minimal CI, but they run (and gate) wherever the
// reference interpreter is available.

/// Run `info complete` under C Tcl 9.0 for a batch of sources.
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
    let oracle = tcl_test_support::witness_tclsh(tcl_dialect::TclVersion::V9_0)?;
    let mut child = Command::new(&oracle.path)
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
    // braces / quotes are balanced and word-separated, C Tcl 9.0
    // `info complete` reflects *only* bracket balance, so the
    // structural index's verdict must match it exactly, both ways:
    //   index.unterminated_count()==0  <=>  info complete == 1.
    let cases = separated_corpus(8000);
    let Some(oracle) = ctcl9_info_complete(&cases) else {
        eprintln!("tclsh9.0 unavailable — skipping C Tcl 9.0 verification");
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
        "index disagreed with C Tcl 9.0 `info complete`: {violations:#?}",
    );
    // The corpus must exercise both verdicts (not a degenerate pass).
    assert!(
        both_complete > 100 && both_incomplete > 100,
        "corpus not exercising both verdicts: complete={both_complete} incomplete={both_incomplete}",
    );
}

#[test]
fn ctcl9_extra_chars_after_word_is_a_documented_divergence() {
    // **Finding (recorded in error-recovery.md):** C Tcl
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
        eprintln!("tclsh9.0 unavailable — skipping C Tcl 9.0 verification");
        return;
    };
    assert_eq!(
        verdicts,
        vec![true, false, false],
        "C Tcl 9.0 semantics drifted"
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
    // *original* deleted position must yield a script C Tcl 9.0
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
        eprintln!("tclsh9.0 unavailable — skipping C Tcl 9.0 verification");
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
        eprintln!("tclsh9.0 unavailable — skipping C Tcl 9.0 verification");
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
        "index found an unterminated `{{` C Tcl 9.0 considers complete: {violations:#?}",
    );
}

#[test]
fn brace_command_sub_interiors_parse_as_scripts() {
    // Closed boundary: `[...]` interiors are parsed recursively as
    // scripts (word-based braces + terminal extra-chars), so the
    // index matches C Tcl 9.0 inside command subs.
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
            assert_eq!(complete, *want, "C Tcl 9.0 semantics drifted for {s:?}");
        }
    }
}

#[test]
fn ctcl9_brace_completeness_is_necessary_with_cmd_subs() {
    // With the recursive command-sub parse, the necessary condition
    // now holds across the *full* adversarial corpus (command subs
    // included): whenever C Tcl 9.0 reports complete, the brace
    // index must report balanced.
    let cases = brace_full_corpus(8000);
    let Some(oracle) = ctcl9_info_complete(&cases) else {
        eprintln!("tclsh9.0 unavailable — skipping C Tcl 9.0 verification");
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
        "index found an unterminated `{{` C Tcl 9.0 considers complete: {violations:#?}",
    );
}

#[test]
fn ctcl9_brace_completeness_iff_on_separated_corpus() {
    // Reference-grounded brace contract: on a corpus where brackets /
    // quotes are balanced and word-separated (and no comments /
    // extra-`}`), C Tcl 9.0 `info complete` reflects only brace
    // balance, so the brace index must match it both ways.
    let cases = brace_separated_corpus(8000);
    let Some(oracle) = ctcl9_info_complete(&cases) else {
        eprintln!("tclsh9.0 unavailable — skipping C Tcl 9.0 verification");
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
        "brace index disagreed with C Tcl 9.0 `info complete`: {violations:#?}",
    );
    assert!(
        both_complete > 100 && both_incomplete > 100,
        "corpus not exercising both verdicts: complete={both_complete} incomplete={both_incomplete}",
    );
}

#[test]
fn brace_semantics_match_reference_examples() {
    // Pin the canonical C Tcl 9.0 brace rules the index must honour.
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
        eprintln!("tclsh9.0 unavailable — skipping C Tcl 9.0 verification");
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

/// C Tcl 9.0 `expr` paren verdict for a batch of expressions.
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
    let oracle = tcl_test_support::witness_tclsh(tcl_dialect::TclVersion::V9_0)?;
    let mut child = Command::new(&oracle.path)
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
        "(", ")", "1", "2", " ", "+", "*", "+1", "*2", "$a", "$arr(1)", "[f]", "\"s)\"", "{x)}",
        "(1+2)", "a", "==", "!",
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
    // The index's paren-balance verdict must match C Tcl 9.0
    // `expr` wherever `expr` gives a *definitive* paren verdict (OK,
    // unbalanced-open, unbalanced-close). Non-paren errors ('X') are
    // excluded — `expr` may hit a different error before reaching the
    // paren check, leaving the paren state indeterminate from the
    // oracle.
    let cases = expr_corpus(8000);
    let Some(oracle) = tcl_expr_paren_verdicts(&cases) else {
        eprintln!("tclsh9.0 unavailable — skipping C Tcl 9.0 verification");
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
        "expr paren index disagreed with C Tcl 9.0: {violations:#?}",
    );
}

#[test]
fn ctcl9_realistic_expr_paren_recovery() {
    // A single forgotten `)` in a real expression: the index
    // predicts the original close site, and the repair evaluates
    // under C Tcl 9.0.
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
        eprintln!("tclsh9.0 unavailable — skipping C Tcl 9.0 verification");
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
fn stdin_chunks_end_where_the_lines_read_complete_commands() {
    assert_eq!(
        stdin_chunk_ends(b"set a 1\nset b {\n2}\nputs \"\xe9"),
        [8, 19]
    );
    assert_eq!(stdin_chunk_ends(b"a; b\nc"), [5, 6]);
    assert_eq!(stdin_chunk_ends(b""), Vec::<usize>::new());
}

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
        "{", "}", "[", "]", "\"", "\\", "$", "a", " ", "\n", ";", "x ", "set ", "puts ", "${v}",
        "{b}", "[a]", "[set x ", "if {1} ", "# c", "\\\n", "\\\\", "${", "1+1",
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
    // The full contract: `script_is_complete` matches C Tcl 9.0
    // `info complete` *both ways* over a general adversarial corpus —
    // no dimension isolation, every construct mixed.
    let cases = general_corpus(12000);
    let Some(oracle) = ctcl9_info_complete(&cases) else {
        eprintln!("tclsh9.0 unavailable — skipping C Tcl 9.0 verification");
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
        "script_is_complete disagreed with C Tcl 9.0 outside the documented \
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

    // The newline in a command substitution whose first command is a
    // comment is also nested.  In particular, the commented-out `]`
    // cannot become a boundary or close the substitution.
    let comment = "set x [\n# ] hidden\nset y 1\n]\nputs done\n";
    let first = u32::try_from(comment.find("]\n").unwrap() + 2).unwrap();
    assert_eq!(
        command_boundaries(comment),
        vec![first, u32::try_from(comment.len()).unwrap()]
    );

    // Raw CRLF is an escaped CR followed by an unescaped LF. It ends the
    // comment, so the first bracket closes the substitution and every
    // following top-level newline is a command boundary.
    let crlf = "set x [\r\n# hidden \\\r\n] still hidden\r\nset y 1\r\n]\nputs done\n";
    assert_eq!(
        command_boundaries(crlf),
        vec![37, 46, 48, u32::try_from(crlf.len()).unwrap()]
    );

    // Escape units, rather than a raw backslash count, determine parity:
    // an even run leaves the newline unescaped and ends the comment.
    let even = "set x [\n# visible close \\\\\n]\nputs done\n";
    let first = u32::try_from(even.find("]\n").unwrap() + 2).unwrap();
    assert_eq!(
        command_boundaries(even),
        vec![first, u32::try_from(even.len()).unwrap()]
    );
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
