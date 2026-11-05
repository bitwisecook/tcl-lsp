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
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Differential harness: the byte-scanned reparse split points against the
//! boundary owner (issue #1786, item 1b).
//!
//! [`tcl_lexer::command_boundaries`] is a **byte scanner**, not a grouper.
//! It answers one question — where may an incremental reparser cut a
//! document into whole top-level commands — and answers it without
//! tokenising, because it is the split-point sibling of
//! [`tcl_lexer::script_is_complete`], the crate's `Tcl_CommandComplete`
//! port, and shares that port's recursive `scan_complete` /
//! `scan_complete_quoted` scanner. Both are registered in
//! `docs/design/contracts/shared-utility-contracts-rust.md` as the
//! *script completeness / reparse windows* surface, separate from the
//! *command / word segmentation* surface [`group_commands`] owns.
//!
//! Two surfaces means the answers must be shown to agree, and until now
//! they were cross-checked only by a seven-string non-straddling assertion
//! in `tcl-compiler`'s segmenter (`command_boundaries_agree_with_segmenter`,
//! removed by this file). This harness replaces it with a corpus
//! differential against the owner itself, over the corpora the other
//! `#1786` differentials use. Agreement with the *compiler segmenter*
//! follows transitively: `differential_group` proves owner ==
//! `segment_commands` command-for-command over the same corpora.
//!
//! # What is proven
//!
//! For a region and a [`LexerConfig`], with `cmds` the owner's top-level
//! commands and `bounds` the scanner's split points:
//!
//! * **containment** — no split point falls strictly inside a command. A
//!   reparse window snapped to a boundary never cuts a command in half.
//! * **coverage** — between two consecutive commands there is a split
//!   point. The scanner never *merges* two commands the owner separates,
//!   which the old straddle-only invariant could not see: a
//!   `command_boundaries` that returned nothing but `source.len()` passed
//!   it. Asserted only where the region is a complete script, which is
//!   the scanner's documented precondition: a split point must be a point
//!   at which the prefix is complete, and inside an unterminated `[`, `{`
//!   or `"` there is no such point. The owner, being a *lexer*, recovers
//!   and keeps grouping there; the scanner, being the `Tcl_CommandComplete`
//!   port, correctly refuses to offer a cut. tcllib's
//!   `page/util_quote.tcl` has a `switch` clause list holding `"\\["`,
//!   which is exactly that shape. Incomplete regions are tallied and
//!   reported, never silently dropped.
//! * **termination** — the last split point is `source.len()`, so the
//!   final command is always inside a window.
//!
//! # The dialect axis, and why it is a *tally* rather than a skip
//!
//! The scanner takes no [`LexerConfig`]; it hard-codes stock Tcl 9
//! structure. Two grammar axes really do move a command boundary, and both
//! are measured, pinned in [`dialect_divergences_are_pinned`], and recorded
//! in the scanner's contract row:
//!
//! * `BraceLineContinuation::Continues` (the F5 trunk) — a newline whose
//!   next line opens with `{` does not terminate the command, so the
//!   scanner splits where the owner does not;
//! * `BracedVarStyle::FirstClose` (the 8.x family) — `${a{b}` names `a{b`
//!   and ends at the first `}`, so the owner splits where the scanner,
//!   nesting `{…}` inside `${…}` the Tcl 9 way, does not.
//!
//! So the corpus walk drives **all three** dialects and, per region, first
//! asks whether that dialect's own grammar puts the commands anywhere
//! other than stock Tcl does. Where it does, the region is counted as a
//! dialect divergence and reported; everywhere else — the overwhelming
//! majority of real corpus text, including every f5 and 8.4 file that
//! happens not to use those two shapes — the three invariants are asserted
//! in full. Nothing is skipped silently: the tally is printed.

use std::fs;
use std::path::{Path, PathBuf};

use tcl_lexer::script::{CommandSpan, group_commands};
use tcl_lexer::{Lexer, LexerConfig, SourceMap, Span, Token, TokenType, command_boundaries};

/// Repo root — two directories above `CARGO_MANIFEST_DIR`.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

/// The three dialects `differential_group` and `differential_segment` use.
type NamedConfig = (&'static str, fn() -> LexerConfig);
const CONFIGS: [NamedConfig; 3] = [
    ("default", LexerConfig::default),
    ("tcl8.4", || LexerConfig::for_dialect("tcl8.4")),
    ("f5-irules", || LexerConfig::for_dialect("f5-irules")),
];

/// Both engines work in the region-local (`base_offset == 0`) space.
fn local_config(config: LexerConfig) -> LexerConfig {
    LexerConfig {
        base_offset: 0,
        base_line: 0,
        base_col: 0,
        ..config
    }
}

fn lex(src: &str, config: LexerConfig) -> Option<Vec<Token>> {
    Lexer::with_source_map(SourceMap::new(src), local_config(config))
        .tokenise_all()
        .ok()
}

/// The owner's commands, each widened over a braced / bracketed final
/// token's closer the way the compiler segmenter's `command_span` does —
/// so containment is asserted against the *whole written* command, not the
/// lexer's inner end.
fn owner_spans(src: &str, config: LexerConfig, tokens: &[Token]) -> Vec<Span> {
    let sm = SourceMap::new(src);
    group_commands(tokens, src, local_config(config))
        .iter()
        .map(|cmd| Span::new(cmd.span.start(), widened_end(&sm, tokens, cmd)))
        .collect()
}

fn widened_end(sm: &SourceMap<'_>, tokens: &[Token], cmd: &CommandSpan) -> u32 {
    let last_word_end = cmd.words.last().map_or(0, |w| w.tokens.end - 1);
    let last = cmd
        .expand_markers
        .last()
        .copied()
        .map_or(last_word_end, |m| m.max(last_word_end));
    let tok = tokens[last];
    if tok.kind.group_closer().is_none() {
        tok.span.end()
    } else {
        tcl_lexer::word_span(sm, tok).end()
    }
}

/// The three invariants over one region. Returns every violation.
///
/// `coverage` is skipped for a region that is not a complete script — see
/// the module docs.
fn violations(src: &str, spans: &[Span], ctx: &str) -> Vec<String> {
    let bounds = command_boundaries(src);
    let len = u32::try_from(src.len()).expect("region fits u32");
    let mut out = Vec::new();

    if bounds.last() != Some(&len) {
        out.push(format!(
            "termination [{ctx}]: last split point {:?} is not the region end {len}",
            bounds.last()
        ));
    }
    for (i, span) in spans.iter().enumerate() {
        if let Some(&b) = bounds.iter().find(|&&b| b > span.start() && b < span.end()) {
            out.push(format!(
                "containment [{ctx}] cmd {i}: split point {b} falls inside command {span:?}",
            ));
        }
    }
    if !tcl_lexer::script_is_complete(src) {
        return out;
    }
    for (i, pair) in spans.windows(2).enumerate() {
        let (prev, next) = (pair[0], pair[1]);
        if !bounds.iter().any(|&b| b >= prev.end() && b <= next.start()) {
            out.push(format!(
                "coverage [{ctx}] cmds {i}/{}: no split point in [{}, {}] — the scanner merges \
                 two commands the owner separates",
                i + 1,
                prev.end(),
                next.start(),
            ));
        }
    }
    out
}

// ---------------------------------------------------------------------
// Edge cases
// ---------------------------------------------------------------------

/// Shapes the scanner and the owner must agree on under stock Tcl. The
/// `differential_group` edge table plus the ones that are specifically
/// about *split points*: nested separators, comments that hide a closer,
/// blank lines, escaped newlines, unterminated delimiters.
const EDGE_CASES: &[&str] = &[
    "",
    "\n",
    "puts hi",
    "puts hi\n",
    "puts hi\n\n",
    "a; b; c",
    "a;;b\n",
    "a\n\n\nb\n",
    "set x 1\nputs $x\n",
    "if {$x} {\n  puts yes\n}\n",
    "proc f {} {\n  return [expr {1 + 2}]\n}\nf\n",
    "namespace eval n {\n  variable v 1\n  proc q {} {}\n}\nn::q\n",
    "set s {a\nb\nc}\nputs $s\n",
    "puts \"a;b\"\nset y [a;b]\n",
    "set x [\n# ] hidden\nset y 1\n]\nputs done\n",
    "# a comment\nputs hi\n",
    "# c1\n# c2\nproc f {} {}\n",
    "# orphan\n\nputs hi\n",
    "puts hi ;# dangling",
    "list a \\\n b\n",
    "set y \"a\\\nb\"\nputs done\n",
    "puts {a}\n",
    "puts {a}b\nputs c\n",
    "puts \"a\"b\nputs c\n",
    "foo {*}$args\n",
    "{a}{*}$b\n",
    "set v ${name}\nputs hi\n",
    "set v $arr(idx)\n",
    "switch $x {\n a {one}\n b {two}\n}\n",
    "set x {a\\}}\nputs done\n",
    // Unterminated / degenerate delimiters: the scanner must stay total and
    // must not invent a split inside the run-to-EOF region.
    "puts {a",
    "puts \"a",
    "puts [a",
    "puts {a\nputs b\n",
    "set x [foo bar\nputs done\n",
    // tcllib `page/util_quote.tcl`: an unterminated `[` inside a quoted
    // word of a `switch` clause list. `info complete` is false from there
    // on, so the scanner offers no further cut and coverage does not apply.
    "\"\\\\[\"  {return 1}\n\"b\" {return 2}\n",
];

#[test]
fn scanner_agrees_with_owner_on_edge_cases() {
    let config = LexerConfig::default();
    let mut failures = Vec::new();
    for &src in EDGE_CASES {
        let Some(tokens) = lex(src, config) else {
            panic!("edge case {src:?} does not lex");
        };
        let spans = owner_spans(src, config, &tokens);
        failures.extend(violations(src, &spans, &format!("default:{src:?}")));
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The two grammar axes the scanner is blind to, pinned with the exact
/// answers both sides give today.
///
/// These are not bugs the harness is hiding: they are the reason
/// `command_boundaries` is registered as its own dialect-blind surface
/// rather than folded onto the owner. If the scanner ever takes a
/// `LexerConfig`, these assertions fail and must be rewritten — which is
/// the point.
#[test]
fn dialect_divergences_are_pinned() {
    // 1. F5 next-line-`{` continuation (`BraceLineContinuation::Continues`).
    //    Stock Tcl: three commands. F5: two — the newline at 7 does not
    //    terminate. The scanner splits at 8 either way.
    let src = "if {$x}\n{\n puts a\n}\nputs done\n";
    assert_eq!(command_boundaries(src), vec![8, 20, 30]);
    assert_eq!(owner_command_count(src, LexerConfig::default()), 3);
    assert_eq!(
        owner_command_count(src, LexerConfig::for_dialect("f5-irules")),
        2
    );
    // Under F5 the scanner cuts the single `if` command in two.
    let cfg = LexerConfig::for_dialect("f5-irules");
    let tokens = lex(src, cfg).expect("lexes");
    let spans = owner_spans(src, cfg, &tokens);
    let broken = violations(src, &spans, "f5-irules");
    assert_eq!(broken.len(), 1, "{broken:?}");
    assert!(broken[0].starts_with("containment"), "{broken:?}");

    // 2. 8.x first-close `${…}` (`BracedVarStyle::FirstClose`). Under 8.4
    //    `${a{b}` names `a{b`, so the newline at 12 is a terminator and the
    //    owner sees two commands; the scanner nests `{…}` inside `${…}` the
    //    Tcl 9 way and sees one, missing the split entirely.
    let src = "set v ${a{b}\nputs hi\n";
    assert_eq!(command_boundaries(src), vec![21]);
    assert_eq!(owner_command_count(src, LexerConfig::default()), 1);
    assert_eq!(
        owner_command_count(src, LexerConfig::for_dialect("tcl8.4")),
        2
    );
    let cfg = LexerConfig::for_dialect("tcl8.4");
    let tokens = lex(src, cfg).expect("lexes");
    let spans = owner_spans(src, cfg, &tokens);
    assert_eq!(spans.len(), 2);
    // No split point anywhere in the gap between the 8.4 commands: the
    // scanner has merged them.
    assert!(
        !command_boundaries(src)
            .iter()
            .any(|&b| b >= spans[0].end() && b <= spans[1].start()),
        "{spans:?}",
    );
    // The blindness reaches the completeness oracle as well — under the
    // Tcl 9 rule this `${…}` never closes, so the whole script reads as
    // incomplete and `command_boundaries` is right, on its own grammar, to
    // offer no cut. That is why the corpus walk asserts coverage only on
    // complete scripts, and why this pin exists instead.
    assert!(!tcl_lexer::script_is_complete(src));
}

fn owner_command_count(src: &str, config: LexerConfig) -> usize {
    let tokens = lex(src, config).expect("lexes");
    group_commands(&tokens, src, local_config(config)).len()
}

// ---------------------------------------------------------------------
// Corpus walk
// ---------------------------------------------------------------------

/// How many levels of `{…}` / `[…]` the corpus walk descends below the top
/// level — `differential_group`'s value, for the same reason: `tokenise_all`
/// is flat, so a top-level-only walk compares almost none of the Tcl in the
/// corpus.
const NEST_DEPTH: usize = 3;

/// Stop after this many violations so a systematic break does not print a
/// megabyte of near-identical failures.
const MAX_FAILURES: usize = 20;

#[derive(Default, Debug, Clone, Copy)]
struct Tally {
    regions: usize,
    commands: usize,
    /// Regions whose owner command spans under this dialect differ from
    /// stock Tcl's — the two axes the scanner is blind to. Asserted
    /// nowhere, reported everywhere.
    dialect_divergent: usize,
    /// Regions that are not complete scripts, where coverage is not
    /// asserted (containment and termination still are).
    incomplete: usize,
}

impl Tally {
    fn add(self, other: Self) -> Self {
        Self {
            regions: self.regions + other.regions,
            commands: self.commands + other.commands,
            dialect_divergent: self.dialect_divergent + other.dialect_divergent,
            incomplete: self.incomplete + other.incomplete,
        }
    }
}

fn walk_region(
    src: &str,
    config: LexerConfig,
    ctx: &str,
    level: usize,
    max_depth: usize,
    tallies: &mut [Tally],
    failures: &mut Vec<String>,
) {
    let Some(tokens) = lex(src, config) else {
        return;
    };
    let spans = owner_spans(src, config, &tokens);
    let tally = &mut tallies[level];
    tally.regions += 1;
    tally.commands += spans.len();
    if !tcl_lexer::script_is_complete(src) {
        tally.incomplete += 1;
    }

    // Does this dialect's grammar put the commands anywhere other than
    // stock Tcl does? If so this region exercises one of the two axes the
    // scanner cannot see, and only the tally records it.
    let stock = LexerConfig::default();
    let stock_spans = lex(src, stock)
        .map(|t| owner_spans(src, stock, &t))
        .unwrap_or_default();
    if spans == stock_spans {
        failures.extend(violations(src, &spans, ctx));
    } else {
        tally.dialect_divergent += 1;
    }

    if level >= max_depth || failures.len() >= MAX_FAILURES {
        return;
    }
    let sm = SourceMap::new(src);
    for &tok in &tokens {
        if !matches!(tok.kind, TokenType::Str | TokenType::Cmd) || tok.content_offset != 1 {
            continue;
        }
        let inner = sm.token_text(tok);
        if inner.trim().is_empty() {
            continue;
        }
        let child_ctx = format!(
            "{ctx} <{}@L{} offset {}>",
            if tok.kind == TokenType::Cmd {
                "[…]"
            } else {
                "{…}"
            },
            level + 1,
            tok.span.start() + 1,
        );
        walk_region(
            inner,
            config,
            &child_ctx,
            level + 1,
            max_depth,
            tallies,
            failures,
        );
        if failures.len() >= MAX_FAILURES {
            return;
        }
    }
}

fn gather(dir: &Path, exts: &[&str], out: &mut Vec<PathBuf>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for entry in rd.flatten() {
        let p = entry.path();
        if p.is_dir() {
            gather(&p, exts, out);
        } else if p
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| exts.contains(&e))
        {
            out.push(p);
        }
    }
}

/// The same corpus roots `differential_group` walks; `tcllib` dominates the
/// cost and so is descended into only by the `--ignored` tier.
fn corpus_files(root: &Path, with_tcllib: bool) -> Vec<PathBuf> {
    let mut files = Vec::new();
    gather(
        &root.join("samples"),
        &["tcl", "irul", "bpftcl"],
        &mut files,
    );
    gather(&root.join("tmp/tcl9.0.4/library"), &["tcl"], &mut files);
    if with_tcllib {
        gather(&root.join("tmp/tcllib-2.0/modules"), &["tcl"], &mut files);
    }
    files.sort();
    files
}

fn run_corpus(files: &[PathBuf], max_depth: usize) -> (usize, Vec<Tally>) {
    let mut checked = 0usize;
    let mut failures: Vec<String> = Vec::new();
    let mut tallies = vec![Tally::default(); max_depth + 1];
    for path in files {
        let Ok(src) = fs::read_to_string(path) else {
            continue;
        };
        checked += 1;
        for (name, make) in CONFIGS {
            let ctx = format!("{name}:{}", path.display());
            walk_region(
                &src,
                make(),
                &ctx,
                0,
                max_depth,
                &mut tallies,
                &mut failures,
            );
        }
        if failures.len() >= MAX_FAILURES {
            break;
        }
    }
    assert!(
        failures.is_empty(),
        "scanner/owner boundary divergence over {checked} corpus files:\n{}",
        failures.join("\n")
    );
    (checked, tallies)
}

fn report(label: &str, checked: usize, tallies: &[Tally]) {
    eprintln!(
        "[differential_boundaries] {label}: {checked} files x 3 dialects: \
         containment + coverage + termination hold"
    );
    for (i, t) in tallies.iter().enumerate() {
        eprintln!(
            "[differential_boundaries]   L{i}: {} regions, {} commands, \
             {} dialect-divergent, {} incomplete (the only ones coverage skips)",
            t.regions, t.commands, t.dialect_divergent, t.incomplete
        );
    }
    let total = tallies.iter().fold(Tally::default(), |a, t| a.add(*t));
    eprintln!(
        "[differential_boundaries]   total: {} regions, {} commands, \
         {} dialect-divergent, {} incomplete",
        total.regions, total.commands, total.dialect_divergent, total.incomplete
    );
}

/// The CI tier: `samples/` and the Tcl 9.0.4 library, three dialects,
/// descending [`NEST_DEPTH`] levels.
#[test]
fn scanner_agrees_with_owner_over_corpora() {
    let root = repo_root();
    let files = corpus_files(&root, false);
    if files.is_empty() {
        eprintln!("[differential_boundaries] skipped: no corpus present");
        return;
    }
    let (checked, tallies) = run_corpus(&files, NEST_DEPTH);
    assert!(checked > 0, "corpus present but no readable files");
    assert!(
        tallies.iter().map(|t| t.commands).sum::<usize>() > 10_000,
        "corpus walk compared too few commands to be a gate"
    );
    report("samples + tcl9.0.4", checked, &tallies);
}

/// The exhaustive tier: the same walk with tcllib included.
/// `docs/design/contracts/test-tiers-and-ci-gates.md` rule 2 — a permanently
/// expensive corpus sweep over `tmp/` belongs behind `--ignored`.
#[test]
#[ignore = "corpus sweep over tcllib; run explicitly with --ignored"]
fn scanner_agrees_with_owner_over_corpora_deep() {
    let files = corpus_files(&repo_root(), true);
    if files.is_empty() {
        eprintln!("[differential_boundaries] skipped: no corpus present");
        return;
    }
    let (checked, tallies) = run_corpus(&files, NEST_DEPTH);
    assert!(checked > 0, "corpus present but no readable files");
    report("samples + tcl9.0.4 + tcllib", checked, &tallies);
}
