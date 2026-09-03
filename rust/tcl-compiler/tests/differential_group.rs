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

//! Differential harness: the shared boundary owner vs the production
//! segmenter (issue #1786, step 2).
//!
//! `tcl_lexer::script::group_commands` is the new single owner of Tcl
//! command and word boundaries.  Before any consumer switches over, it has
//! to be shown to reproduce — command for command, word for word — what the
//! shipping segmenter (`segment_commands_with_offset_and_config`, 299 call
//! sites, every LSP diagnostic) already answers.
//!
//! Compared per source, per dialect:
//!
//! * command count and whole-command spans;
//! * per-word spans;
//! * per-word kind, derived independently on each side (the segmenter has
//!   no `WordKind` field, so it is reconstructed from its own `argv` /
//!   `single_token_word` outputs by `runtime/rust`'s rule);
//! * per-word single-token flags;
//! * per-word `{*}` expansion flags;
//! * the preceding comment.
//!
//! # The two adaptations, and why they are not papering over anything
//!
//! **1. Command-span widening (representational).**  The owner reports the
//! lexer's *token* span: a final braced or bracketed word ends at the
//! inner-end convention, one byte before its `}` / `]`.  The segmenter
//! applies `command_span` / `widen_word_end`
//! (`segmenter.rs:320-325`) on top, widening that final token over its
//! closer — deliberately *not* for a quoted `"…"` last word, because
//! `cmd.range` consumers depend on the inner end there.  That is a
//! presentation policy layered on the same boundary, not a different
//! boundary, so this harness applies the identical policy (through
//! `tcl_lexer::word_span`, the shared owner of the widening) to the owner's
//! end before comparing.  Nothing else about either span is touched.
//!
//! **2. The F5 `else` / `elseif` lookahead (behavioural, and deliberately
//! not in the owner).**  Under the F5 trunk grammar only
//! (`brace_line_continuation.continues()`, i.e. the `f5-irules` dialect),
//! `segment_commands_local` runs `merge_f5_if_else_lookahead` as a
//! *post-pass*: an `else` / `elseif` command one single newline after an
//! `if` is folded back into that `if` (measurements N5 — the lookahead is
//! performed by `if` itself, not by the lexical grammar).  It is a
//! command-level rule the compiler owns and, per the #1786 staging, keeps
//! owning; putting it in the lexer would make the compiler apply it twice
//! once `build.rs` consumes the owner.  So the harness replays that exact
//! post-pass on the owner's side for the `f5-irules` config, using the
//! segmenter's own predicate (`word_piece` texts + `single_newline_gap`).
//! Every other boundary question is compared unadapted.
//!
//! # Nested bodies
//!
//! `Lexer::tokenise_all` is flat.  A `proc p {} { … }` body is a single
//! `Str` token, so a corpus walk that only groups the *top* level compares
//! almost none of the Tcl in the corpus: measured over `samples/`, the Tcl
//! 9.0.4 library and tcllib 2.0 — 965 files — top-level grouping yields
//! **zero** `{*}` markers, even though `{*}` is written in many of them.
//! That is exactly the boundary #1786 next changes in `runtime/rust`, so
//! the corpus walk also descends [`NEST_DEPTH`] levels of `{…}` bodies and
//! `[…]` command substitutions ([`walk_region`]), driving *both* sides over
//! the identical inner text under the identical [`LexerConfig`].

use std::fs;
use std::path::{Path, PathBuf};

use tcl_lexer::script::{CommandSpan, WordKind, group_commands};
use tcl_lexer::{Lexer, LexerConfig, SourceMap, Span, Token, TokenType};

use tcl_compiler::segmenter::{
    SegmentedCommand, segment_commands_with_offset_and_config, word_piece,
};

/// Repo root — two directories above `CARGO_MANIFEST_DIR`.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

// ---------------------------------------------------------------------
// The owner side
// ---------------------------------------------------------------------

/// A flattened, comparable view of one command's boundaries.  Both sides
/// are reduced to this so a mismatch names a field, not a struct dump.
#[derive(Debug, Clone, PartialEq, Eq)]
struct CmdView {
    span: Span,
    words: Vec<WordView>,
    comment: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WordView {
    span: Span,
    kind: WordKind,
    single: bool,
    expand: bool,
}

/// The segmenter's `widen_word_end`: widen a braced / bracketed token over
/// its closer, leaving every other kind (quoted `"…"` included) at the
/// inner end.  Routed through `tcl_lexer::word_span`, the same shared owner
/// the production code uses, so this cannot drift from it.
fn widen(tok: Token, sm: &SourceMap<'_>) -> u32 {
    if tok.kind.group_closer().is_none() {
        return tok.span.end();
    }
    tcl_lexer::word_span(sm, tok).end()
}

/// The word's `SegmentedCommand::texts` spelling — the concatenated
/// `word_piece` of its fragments.  Used only by the F5 merge predicate,
/// which keys off `cmd.name()`.
fn owner_word_text(sm: &SourceMap<'_>, tokens: &[Token], word: &tcl_lexer::WordSpan) -> String {
    word.tokens
        .clone()
        .map(|i| word_piece(sm, tokens[i]))
        .collect()
}

/// The region-local view of `config`: both engines group and segment in the
/// `base_offset == 0` space (`group_commands` is documented to require it,
/// and `segment_commands_with_offset_and_config` reaches a nonzero base only
/// by shifting its local result), so every comparison here is driven at zero.
fn local_config(config: LexerConfig) -> LexerConfig {
    LexerConfig {
        base_offset: 0,
        base_line: 0,
        base_col: 0,
        ..config
    }
}

/// Lex `src` the way [`owner`] does — the token stream `check` compares from.
fn owner_lex(src: &str, config: LexerConfig) -> Option<Vec<Token>> {
    Lexer::with_source_map(SourceMap::new(src), local_config(config))
        .tokenise_all()
        .ok()
}

/// Run the owner over `src` and reduce it to the comparable view, applying
/// the two adaptations documented at the top of this file.
fn owner(src: &str, config: LexerConfig) -> Vec<CmdView> {
    // The segmenter degrades to an empty document on a hard LexError (only
    // reachable under `strict_quoting`, which it never sets).
    let Some(tokens) = owner_lex(src, config) else {
        return Vec::new();
    };
    owner_views(src, config, &tokens)
}

/// [`owner`] over an already-lexed stream.  The corpus walk holds the lex
/// it needs for descent anyway, so reusing it keeps the deepened walk from
/// lexing every nested body a third time.
fn owner_views(src: &str, config: LexerConfig, tokens: &[Token]) -> Vec<CmdView> {
    let config = local_config(config);
    let sm = SourceMap::new(src);
    let grouped = group_commands(tokens, src, config);

    let mut views: Vec<CmdView> = grouped
        .iter()
        .map(|cmd| CmdView {
            // Adaptation 1: the segmenter's final-token widening policy.
            span: Span::new(cmd.span.start(), widen(tokens[last_content(cmd)], &sm)),
            words: cmd
                .words
                .iter()
                .map(|w| WordView {
                    span: w.span,
                    kind: w.kind,
                    single: w.is_single_token(),
                    expand: w.expand,
                })
                .collect(),
            comment: cmd.comment_text(tokens, src),
        })
        .collect();

    if config.brace_line_continuation.continues() {
        // Adaptation 2: replay the compiler's N5 `else` lookahead post-pass.
        views = merge_f5_if_else(src, &sm, tokens, &grouped, views);
    }
    views
}

/// Index of the command's last content token — the one `command_span`
/// widens.  `{*}` markers count: they are in the segmenter's `all_tokens`.
fn last_content(cmd: &CommandSpan) -> usize {
    let last_word_end = cmd.words.last().map_or(0, |w| w.tokens.end - 1);
    cmd.expand_markers
        .last()
        .copied()
        .map_or(last_word_end, |m| m.max(last_word_end))
}

/// The gap between two commands is exactly one newline plus horizontal
/// whitespace (a verbatim copy of `segmenter::single_newline_gap`).
fn single_newline_gap(source: &str, gap_start: u32, gap_end: u32) -> bool {
    let Some(gap) = source.get(gap_start as usize..gap_end as usize) else {
        return false;
    };
    let mut newlines = 0usize;
    for byte in gap.bytes() {
        match byte {
            b'\n' => newlines += 1,
            b' ' | b'\t' | b'\r' => {}
            _ => return false,
        }
    }
    newlines == 1
}

/// `segmenter::merge_f5_if_else_lookahead`, replayed over the owner's
/// output.  Same predicate, same fold: the `else` command's words become
/// further `if` arguments, the `if` command's span stretches to cover them,
/// and the `else`'s own preceding comment is dropped.
fn merge_f5_if_else(
    src: &str,
    sm: &SourceMap<'_>,
    tokens: &[Token],
    grouped: &[CommandSpan],
    views: Vec<CmdView>,
) -> Vec<CmdView> {
    let names: Vec<String> = grouped
        .iter()
        .map(|c| {
            c.words
                .first()
                .map_or_else(String::new, |w| owner_word_text(sm, tokens, w))
        })
        .collect();
    let mut out: Vec<CmdView> = Vec::with_capacity(views.len());
    let mut prev_index: Option<usize> = None;
    for (i, view) in views.into_iter().enumerate() {
        let first = &grouped[i].words[0];
        let mergeable = matches!(names[i].as_str(), "else" | "elseif")
            && tokens[first.tokens.start].kind == TokenType::Esc
            && first.is_single_token()
            && prev_index.is_some_and(|p| {
                names[p] == "if"
                    && single_newline_gap(
                        src,
                        out.last()
                            .expect("prev_index implies a pushed view")
                            .span
                            .end(),
                        view.span.start(),
                    )
            });
        if !mergeable {
            prev_index = Some(i);
            out.push(view);
            continue;
        }
        let prev = out.last_mut().expect("mergeable requires a predecessor");
        prev.span = Span::new(prev.span.start(), view.span.end());
        prev.words.extend(view.words);
    }
    out
}

// ---------------------------------------------------------------------
// The segmenter side
// ---------------------------------------------------------------------

/// Reduce the production segmenter's output to the same view.
///
/// The word *kind* has no field in `SegmentedCommand`, so it is
/// reconstructed here from the segmenter's own outputs by `runtime/rust`'s
/// rule (`parse.rs` `build_word`): a single `Str` word is braced, a word
/// whose first source byte is `"` is quoted, everything else is bare.  That
/// keeps the two sides' kinds independently derived.
fn segmenter(src: &str, config: LexerConfig) -> Vec<CmdView> {
    segment_commands_with_offset_and_config(src, 0, config)
        .iter()
        .map(|cmd| CmdView {
            span: cmd.span,
            words: (0..cmd.argv.len()).map(|i| seg_word(src, cmd, i)).collect(),
            comment: cmd.preceding_comment.clone(),
        })
        .collect()
}

fn seg_word(src: &str, cmd: &SegmentedCommand, i: usize) -> WordView {
    let tok = cmd.argv[i];
    let single = cmd.single_token_word[i];
    let kind = if single && tok.kind == TokenType::Str {
        WordKind::Braced
    } else if src.as_bytes().get(tok.span.start() as usize) == Some(&b'"') {
        WordKind::Quoted
    } else {
        WordKind::Bare
    };
    WordView {
        span: tok.span,
        kind,
        single,
        expand: cmd
            .expand_word
            .as_ref()
            .and_then(|e| e.get(i).copied())
            .unwrap_or(false),
    }
}

// ---------------------------------------------------------------------
// The comparison
// ---------------------------------------------------------------------

/// Compare both sides over `src` under `config`.
///
/// `tokens` is the owner-side lex of `src`, when the caller already holds
/// one (the corpus walk does — it lexes each region to find the nested
/// regions inside it); `None` lexes here.  Either way both sides see the
/// same `src` and the same `config`: the segmenter is always driven through
/// `segment_commands_with_offset_and_config` on that same text, so there is
/// no third path.
fn check(
    src: &str,
    config: LexerConfig,
    ctx: &str,
    tokens: Option<&[Token]>,
) -> Result<(), String> {
    let o = match tokens {
        Some(tokens) => owner_views(src, config, tokens),
        None => owner(src, config),
    };
    let s = segmenter(src, config);
    if o.len() != s.len() {
        return Err(format!(
            "command count [{ctx}]: owner {} vs segmenter {}",
            o.len(),
            s.len()
        ));
    }
    for (i, (oc, sc)) in o.iter().zip(s.iter()).enumerate() {
        if oc.span != sc.span {
            return Err(format!(
                "command span [{ctx}] cmd {i}: owner {:?} vs segmenter {:?}",
                oc.span, sc.span
            ));
        }
        if oc.words.len() != sc.words.len() {
            return Err(format!(
                "word count [{ctx}] cmd {i}: owner {} vs segmenter {} ({:?} / {:?})",
                oc.words.len(),
                sc.words.len(),
                oc.words,
                sc.words
            ));
        }
        for (j, (ow, sw)) in oc.words.iter().zip(sc.words.iter()).enumerate() {
            if ow.span != sw.span {
                return Err(format!(
                    "word span [{ctx}] cmd {i} word {j}: owner {:?} vs segmenter {:?}",
                    ow.span, sw.span
                ));
            }
            if ow.kind != sw.kind {
                return Err(format!(
                    "word kind [{ctx}] cmd {i} word {j}: owner {:?} vs segmenter {:?}",
                    ow.kind, sw.kind
                ));
            }
            if ow.single != sw.single {
                return Err(format!(
                    "single-token [{ctx}] cmd {i} word {j}: owner {} vs segmenter {}",
                    ow.single, sw.single
                ));
            }
            if ow.expand != sw.expand {
                return Err(format!(
                    "expand [{ctx}] cmd {i} word {j}: owner {} vs segmenter {}",
                    ow.expand, sw.expand
                ));
            }
        }
        if oc.comment != sc.comment {
            return Err(format!(
                "preceding comment [{ctx}] cmd {i}: owner {:?} vs segmenter {:?}",
                oc.comment, sc.comment
            ));
        }
    }
    Ok(())
}

/// A named dialect and the constructor for its [`LexerConfig`].
type NamedConfig = (&'static str, fn() -> LexerConfig);

/// The three dialects `differential_segment` uses.
const CONFIGS: [NamedConfig; 3] = [
    ("default", LexerConfig::default),
    ("tcl8.4", || LexerConfig::for_dialect("tcl8.4")),
    ("f5-irules", || LexerConfig::for_dialect("f5-irules")),
];

/// The `differential_segment` edge-case table plus the `{*}`-after-brace
/// and welding shapes this owner exists to settle.
const EDGE_CASES: &[&str] = &[
    "",
    "puts hi",
    "puts hi\n",
    "puts hi\n\n",
    "set x 1\nputs $x\n",
    "if {$x} {body}",
    "if {$x} {\n  puts yes\n}\n",
    "proc f {} {}",
    "set x {}",
    "set x {}\n",
    "a {}}",
    "puts \"hello\"",
    "puts \"hello\"\n",
    "puts \"hi\" tail",
    "foo {*}$args",
    "foo {*}{*}$args",
    "foo {*}",
    "list a \\\n b",
    "a; b; c",
    "puts [expr {1 + 2}]",
    "set x \"a\nb\nc\"",
    "  indented puts hi  \n",
    "# a comment\nputs hi\n",
    "# c1\n# c2\nproc f {} {}\n",
    "# orphan\n\nputs hi\n",
    "puts hi ;# dangling",
    "set v $arr(idx)",
    "set v ${name}",
    "puts $a$b",
    "namespace eval ns {\n  proc g {} {}\n}\n",
    "switch $x {\n a {one}\n b {two}\n}\n",
    "puts \"\\\n\"",
    "puts \"$x\\\n\"",
    "set y \"a\\\nb\"",
    // #1786: `{*}` welded to a close-brace — the measured divergence.
    "{a}{*}$b",
    "{a}{*}b",
    "set x {a}{*}$y",
    "foo \"q\"{*}$z",
    "foo {*}$b",
    "{a}{*}{b}",
    "puts {a}{*}$b tail",
    // Welding without `{*}`.
    "puts {a}b",
    "puts {a}{b}",
    "puts {a}$b",
    "puts {a}[b]",
    "puts {}x",
    "puts \"a\"b",
    // Unterminated / degenerate delimiters — the lexer stays lenient.
    "puts {a",
    "puts \"a",
    "puts [a",
    "set x {a\\}}",
    "puts {a}\n",
    // F5 N-rules: the brace-continuation SEP and the `else` lookahead.
    "if {$x}\n{\n puts a\n}\n",
    "if {$x} {\n a\n}\nelse {\n b\n}\n",
    "if {$x} {\n a\n}\n\nelse {\n b\n}\n",
    "if {$x} {a}\nelseif {$y} {b}\nelse {c}\n",
];

#[test]
fn owner_matches_segmenter_on_edge_cases() {
    for &src in EDGE_CASES {
        for (name, make) in CONFIGS {
            let config = make();
            let ctx = format!("{name}:{src:?}");
            if let Err(msg) = check(src, config, &ctx, None) {
                panic!("{msg}");
            }
        }
    }
}

/// The `{*}`-after-close-brace shapes, pinned explicitly: the owner takes
/// the segmenter's boundary (finish the word at `{*}`), which is the one
/// `runtime/rust` did *not* take, and additionally flags the weld.
#[test]
fn expand_after_close_brace_takes_the_segmenter_boundary() {
    let cases: &[(&str, usize)] = &[
        ("{a}{*}$b", 2),
        ("{a}{*}b", 2),
        ("set x {a}{*}$y", 4),
        ("foo \"q\"{*}$z", 2),
        ("foo {*}$b", 2),
    ];
    for &(src, words) in cases {
        let config = LexerConfig::default();
        let tokens = Lexer::with_config(src, config).tokenise_all().unwrap();
        let grouped = group_commands(&tokens, src, config);
        assert_eq!(grouped.len(), 1, "{src:?}");
        assert_eq!(grouped[0].words.len(), words, "{src:?}");
        assert_eq!(
            grouped[0].words.len(),
            segment_commands_with_offset_and_config(src, 0, config)[0]
                .argv
                .len(),
            "{src:?}: owner and segmenter must agree on word count"
        );
    }

    // The weld flag is set exactly where C says `extra characters after
    // close-brace`, and nowhere else.
    for (src, expected) in [
        ("{a}{*}$b", vec![true, false]),
        ("{a}{*}b", vec![true, false]),
        ("foo \"q\"{*}$z", vec![false, false]),
        ("foo {*}$b", vec![false, false]),
    ] {
        let config = LexerConfig::default();
        let tokens = Lexer::with_config(src, config).tokenise_all().unwrap();
        let grouped = group_commands(&tokens, src, config);
        let flags: Vec<bool> = grouped[0]
            .words
            .iter()
            .map(|w| w.welded_after_close)
            .collect();
        assert_eq!(flags, expected, "{src:?}");
    }
}

// ---------------------------------------------------------------------
// Corpus walk
// ---------------------------------------------------------------------

/// How many levels of `{…}` / `[…]` the corpus walk descends below the
/// top level.
///
/// `Lexer::tokenise_all` is flat: a `proc p {} { … }` body is one `Str`
/// token and nothing inside it is ever grouped, so a top-level-only corpus
/// walk compares almost none of the real Tcl in the corpus — measurably,
/// **zero** `{*}` expansions across all 965 files, even though `{*}` is
/// written in many of them.  Descending three levels is what puts a
/// `proc` body, a `foreach`/`if` body inside it, and one more level under
/// that in front of both engines.
const NEST_DEPTH: usize = 3;

/// Stop after this many divergences so a systematic break does not print a
/// megabyte of near-identical failures.
const MAX_FAILURES: usize = 20;

/// What one nesting level of the corpus walk compared.
#[derive(Default, Debug, Clone, Copy)]
struct Tally {
    /// Regions compared (files at level 0, nested bodies below).
    regions: usize,
    commands: usize,
    words: usize,
    /// `{*}` markers the lexer produced in this level's regions — the
    /// number this walk exists to move off zero.
    expands: usize,
}

impl Tally {
    fn add(self, other: Self) -> Self {
        Self {
            regions: self.regions + other.regions,
            commands: self.commands + other.commands,
            words: self.words + other.words,
            expands: self.expands + other.expands,
        }
    }
}

/// Compare one region with [`check`], tally it, and descend.
///
/// `sm` maps the region and carries the base offset of its first byte, so
/// `range_positions` resolves an inner token to its position in the
/// *original file* and a failure names where to look.  The comparison
/// itself runs in the region-local (`base_offset == 0`) space both engines
/// require: `group_commands` is documented to take a local token stream,
/// and `segment_commands_with_offset_and_config` reaches a nonzero base
/// only by shifting its local result, so a nonzero base would add the same
/// constant to both sides and prove nothing extra.  Both sides therefore
/// get the identical inner `&str` and the identical `LexerConfig` — no
/// third path, exactly as at the top level.
///
/// A descended region is a `{…}` or `[…]` token's inner text, taken as
/// `SourceMap::token_text` — a verbatim slice of the region, delimiters
/// excluded, anchored `content_offset` bytes past the opener, the same
/// anchor `parsing::syntax::descend::descend_token` uses.  It is taken from
/// the *token*, never from a reconstructed word value, so the
/// `contiguous_prefix` / `body_text_in_region` clamp a compound `{body}x`
/// word needs does not arise here: a token's text is a source slice by
/// construction.
///
/// Every braced group is descended, not just registry-resolved bodies: a
/// differential harness only needs both engines to see the same text, and
/// a `switch` clause list or a braced list is as good a boundary exercise
/// as a `proc` body.
fn walk_region(
    sm: &SourceMap<'_>,
    config: LexerConfig,
    ctx: &str,
    level: usize,
    max_depth: usize,
    tallies: &mut [Tally],
    failures: &mut Vec<String>,
) {
    let src = sm.source();
    let Some(tokens) = owner_lex(src, config) else {
        return;
    };
    if let Err(msg) = check(src, config, ctx, Some(&tokens)) {
        failures.push(msg);
    }
    let grouped = group_commands(&tokens, src, local_config(config));
    let tally = &mut tallies[level];
    tally.regions += 1;
    tally.commands += grouped.len();
    tally.words += grouped.iter().map(|c| c.words.len()).sum::<usize>();
    tally.expands += tokens
        .iter()
        .filter(|t| t.kind == TokenType::Expand)
        .count();

    if level >= max_depth || failures.len() >= MAX_FAILURES {
        return;
    }
    for &tok in &tokens {
        // A real `{…}` / `[…]`: the lexer sets `content_offset == 1` for
        // the opener it stripped.  Anything else (a bare word, a `"…"`
        // fragment, a recovery token) is not a delimited script region.
        if !matches!(tok.kind, TokenType::Str | TokenType::Cmd) || tok.content_offset != 1 {
            continue;
        }
        let inner = sm.token_text(tok);
        if inner.trim().is_empty() {
            continue;
        }
        let (start, _) = sm.range_positions(tok.span);
        let child = SourceMap::new(inner).with_base(
            start.offset + 1,
            start.line,
            start.character.get() + 1,
        );
        let child_ctx = format!(
            "{ctx} <{}@L{} line {} col {} offset {}>",
            if tok.kind == TokenType::Cmd {
                "[…]"
            } else {
                "{…}"
            },
            level + 1,
            start.line + 1,
            start.character.get() + 2,
            child.base_offset(),
        );
        walk_region(
            &child,
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

/// Corpus roots. `tcllib` is by far the largest (56M vs 7.5M for the other
/// two together) and dominates the cost of a nested walk, so the two tiers
/// below differ only in whether it is descended into.
fn corpus_files(root: &std::path::Path, with_tcllib: bool) -> Vec<PathBuf> {
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

/// Compare every file in `files` at every dialect, descending `max_depth`
/// levels into `{…}` / `[…]` bodies.
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
            // Level 0 is the original top-level comparison, unchanged —
            // same `check`, same context string.  Levels 1..=max_depth are
            // the added nested coverage.
            let ctx = format!("{name}:{}", path.display());
            walk_region(
                &SourceMap::new(&src),
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
        "owner/segmenter divergence over {checked} corpus files:\n{}",
        failures.join("\n")
    );
    (checked, tallies)
}

fn report(label: &str, checked: usize, tallies: &[Tally]) {
    let top = tallies[0];
    let nested = tallies[1..]
        .iter()
        .fold(Tally::default(), |acc, t| acc.add(*t));
    eprintln!("[differential_group] {label}: {checked} files x 3 dialects: owner == segmenter");
    eprintln!(
        "[differential_group]   top level: {} regions, {} commands, {} words, {} {{*}}",
        top.regions, top.commands, top.words, top.expands
    );
    for (i, t) in tallies[1..].iter().enumerate() {
        eprintln!(
            "[differential_group]   nested L{}: {} bodies, {} commands, {} words, {} {{*}}",
            i + 1,
            t.regions,
            t.commands,
            t.words,
            t.expands
        );
    }
    eprintln!(
        "[differential_group]   nested total: {} bodies, {} commands, {} words, {} {{*}}",
        nested.regions, nested.commands, nested.words, nested.expands
    );
}

/// The CI tier: top-level grouping over **every** corpus exactly as before,
/// plus nested bodies in `samples/` and the Tcl 9.0.4 library.
///
/// Nested coverage is what makes this harness able to see `{*}` at all — at
/// top level the corpus contains **zero** `Expand` tokens, because
/// `tokenise_all` does not descend into braced bodies. Dropping it from CI
/// would leave the `{*}` behaviour change in `runtime/rust::parse.rs`
/// guarded only by this file's literal edge-case table.
///
/// tcllib is descended into only by
/// [`owner_matches_segmenter_over_corpora_deep`]: it is 56M against 7.5M for
/// the other two and dominates the cost, while the `{*}` shapes it adds are
/// the same ones `samples/` and the 9.0.4 library already cover.
#[test]
fn owner_matches_segmenter_over_corpora() {
    let root = repo_root();
    let all = corpus_files(&root, true);
    if all.is_empty() {
        eprintln!("[differential_group] skipped: no corpus present");
        return;
    }
    // Unchanged from before the nested walk existed: every corpus, top level.
    let (checked, top_tallies) = run_corpus(&all, 0);
    assert!(checked > 0, "corpus present but no readable files");
    report("top level, all corpora", checked, &top_tallies);

    // The added coverage, on the two corpora that are cheap to descend.
    let (nested_checked, tallies) = run_corpus(&corpus_files(&root, false), NEST_DEPTH);
    assert!(
        tallies[1..].iter().any(|t| t.expands > 0),
        "the nested walk must exercise `{{*}}` from the corpus (top level saw 0)"
    );
    report("nested, samples + tcl9.0.4", nested_checked, &tallies);
}

/// The exhaustive tier: the same walk with tcllib included.
///
/// `docs/design/contracts/test-tiers-and-ci-gates.md` rule 2 — a permanently
/// expensive corpus sweep over `tmp/tcl*` and tcllib belongs behind
/// `--ignored`, never in `prep-pr`, `test`, `check-all` or CI. Measured: 58s
/// for this test against 8s for the CI tier above, and 629 `{*}` sites
/// against 55 — tcllib is 92% of the cost for shapes already covered.
#[test]
#[ignore = "corpus sweep over tcllib; run explicitly with --ignored"]
fn owner_matches_segmenter_over_corpora_deep() {
    let files = corpus_files(&repo_root(), true);
    if files.is_empty() {
        eprintln!("[differential_group] skipped: no corpus present");
        return;
    }
    let (checked, tallies) = run_corpus(&files, NEST_DEPTH);
    assert!(checked > 0, "corpus present but no readable files");
    report("samples + tcl9.0.4 + tcllib", checked, &tallies);
}
