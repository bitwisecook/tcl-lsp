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

//! Differential harness: CST-derived segmenter vs the token-loop oracle.
//!
//! `CST-PORT` strip 5 verification.  The canonical red-green CST
//! (`parsing::syntax::{build,red,segment}`) must derive a
//! `SegmentedCommand` list **byte-identical** to the current token-loop
//! `segment_commands_local` (the oracle), field-for-field, plus:
//!
//! - **losslessness** — the CST's reconstructed text equals the source;
//! - **position-equivalence** — the red fragment tokens reproduce the
//!   lexer's exact token spans (covered transitively by `all_tokens`
//!   parity, since each CST `all_tokens` entry is a red `to_token()`).
//!
//! Verified over a crafted edge-case table and, when present, the
//! `tmp/tcl{8.4,8.5,8.6,9.0}` source corpus.  The harness is the gate the
//! task requires green *before* the segmenter's internals are flipped to
//! the CST; it stays as a permanent regression net afterwards.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use tcl_lexer::LexerConfig;

use tcl_compiler::parsing::syntax::build::build_document;
use tcl_compiler::parsing::syntax::red::SyntaxTree;
use tcl_compiler::segmenter::{SegmentedCommand, segment_commands_with_offset_and_config};

/// Repo root — two directories above `CARGO_MANIFEST_DIR`.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

/// The **independent oracle**: a frozen, byte-for-byte copy of the
/// pre-CST token-loop `segment_commands_local`, snapshotted from the last
/// state before the segmenter switched to CST derivation.
///
/// This is what makes the harness a genuine differential rather than a
/// self-comparison: the production segmenter
/// ([`production`]) now derives from the CST, so comparing it against the
/// live `segment_commands_local` would be tautological.  The frozen loop
/// is the original reference behaviour the CST port must reproduce.
fn oracle(src: &str, config: LexerConfig) -> Vec<SegmentedCommand> {
    frozen_oracle::segment_local(src, config)
}

/// The **production** segmenter under test (CST-backed, local-offset
/// space).  Routes through the real public entry point so the test
/// exercises the shipping code path, not a re-implementation.
fn production(src: &str, config: LexerConfig) -> Vec<SegmentedCommand> {
    segment_commands_with_offset_and_config(src, 0, config)
}

/// Assert two `SegmentedCommand`s are equal field-for-field.
fn assert_seg_eq(o: &SegmentedCommand, c: &SegmentedCommand, ctx: &str) {
    assert_eq!(o.span, c.span, "span mismatch [{ctx}]");
    assert_eq!(o.argv, c.argv, "argv mismatch [{ctx}]");
    assert_eq!(o.texts, c.texts, "texts mismatch [{ctx}]");
    assert_eq!(
        o.word_fragments, c.word_fragments,
        "word_fragments mismatch [{ctx}]"
    );
    assert_eq!(
        o.single_token_word, c.single_token_word,
        "single_token_word mismatch [{ctx}]"
    );
    assert_eq!(o.all_tokens, c.all_tokens, "all_tokens mismatch [{ctx}]");
    assert_eq!(o.is_partial, c.is_partial, "is_partial mismatch [{ctx}]");
    assert_eq!(o.expand_word, c.expand_word, "expand_word mismatch [{ctx}]");
    assert_eq!(
        o.preceding_comment, c.preceding_comment,
        "preceding_comment mismatch [{ctx}]"
    );
}

/// Compare the production (CST) segmenter against the frozen oracle over
/// `src` for a given dialect config, and check losslessness.  Returns
/// `Ok(())` or a descriptive error string.
fn check(src: &str, config: LexerConfig, ctx: &str) -> Result<(), String> {
    let o = oracle(src, config);
    let c = production(src, config);
    if o.len() != c.len() {
        return Err(format!(
            "command count mismatch [{ctx}]: oracle {} vs production {}",
            o.len(),
            c.len()
        ));
    }
    for (i, (oc, cc)) in o.iter().zip(c.iter()).enumerate() {
        // Use catch via manual field comparison so the corpus sweep can
        // accumulate rather than panicking on the first file.
        if oc.span != cc.span {
            return Err(format!(
                "span mismatch [{ctx}] cmd {i}: {:?} vs {:?}",
                oc.span, cc.span
            ));
        }
        if oc.argv != cc.argv {
            return Err(format!("argv mismatch [{ctx}] cmd {i}"));
        }
        if oc.texts != cc.texts {
            return Err(format!(
                "texts mismatch [{ctx}] cmd {i}: {:?} vs {:?}",
                oc.texts, cc.texts
            ));
        }
        if oc.word_fragments != cc.word_fragments {
            return Err(format!("word_fragments mismatch [{ctx}] cmd {i}"));
        }
        if oc.single_token_word != cc.single_token_word {
            return Err(format!("single_token_word mismatch [{ctx}] cmd {i}"));
        }
        if oc.all_tokens != cc.all_tokens {
            return Err(format!("all_tokens mismatch [{ctx}] cmd {i}"));
        }
        if oc.is_partial != cc.is_partial {
            return Err(format!("is_partial mismatch [{ctx}] cmd {i}"));
        }
        if oc.expand_word != cc.expand_word {
            return Err(format!("expand_word mismatch [{ctx}] cmd {i}"));
        }
        if oc.preceding_comment != cc.preceding_comment {
            return Err(format!("preceding_comment mismatch [{ctx}] cmd {i}"));
        }
    }
    // Losslessness: the anchored tree's text reproduces the source.
    let (doc, _) = build_document(src, config);
    let tree = SyntaxTree::new(doc);
    if tree.text() != src {
        return Err(format!("losslessness mismatch [{ctx}]"));
    }
    Ok(())
}

/// A spread of crafted sources exercising the segmenter's quirks.
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
    "a {}}",          // empty {} followed by a stray `}` word
    "puts \"hello\"", // quoted last word — the closer-convention edge
    "puts \"hello\"\n",
    "puts \"hi\" tail", // quoted non-last word
    "foo {*}$args",     // expand
    "foo {*}{*}$args",  // double expand
    "foo {*}",          // trailing `{*}` is a literal Str, not an expand
    "list a \\\n b",    // backslash-newline continuation
    "a; b; c",          // semicolon separators
    "puts [expr {1 + 2}]",
    "set x \"a\nb\nc\"", // multi-line quoted
    "  indented puts hi  \n",
    "# a comment\nputs hi\n",
    "# c1\n# c2\nproc f {} {}\n",
    "# orphan\n\nputs hi\n", // blank-line comment reset
    "puts hi ;# dangling",   // dangling trailing comment
    "set v $arr(idx)",
    "set v ${name}",
    "puts $a$b", // compound var word
    "namespace eval ns {\n  proc g {} {}\n}\n",
    "switch $x {\n a {one}\n b {two}\n}\n",
    // Quoted whole-content line-continuation: the quoted word is a single
    // ESC whose text is exactly "\\\n". Per C Tcl this is a real
    // fragment of the quoted word (it collapses to a space at substitution
    // time), so both the frozen oracle and the CST keep it as a word rather
    // than dropping it — see build.rs's backslash-newline note.
    "puts \"\\\n\"",
    "puts \"$x\\\n\"",
    "set y \"a\\\nb\"", // partial continuation — also a quoted-content ESC fragment
];

#[test]
fn cst_matches_oracle_on_edge_cases() {
    for &src in EDGE_CASES {
        for (name, config) in [
            ("default", LexerConfig::default()),
            ("tcl8.4", LexerConfig::for_dialect("tcl8.4")),
            ("f5-irules", LexerConfig::for_dialect("f5-irules")),
        ] {
            let ctx = format!("{name}:{src:?}");
            if let Err(msg) = check(src, config, &ctx) {
                panic!("{msg}");
            }
        }
    }
}

#[test]
fn production_matches_oracle_with_nonzero_base_offset() {
    // The frozen oracle derives in local space; the production
    // `_with_offset` API relocates via `shifted_by`.  Shifting the frozen
    // oracle's local output by the same base must reproduce the
    // production segmenter's offset output — a genuine cross-check of the
    // relocation path.
    let src = "if {$x} {body}\nputs \"q\"\n";
    let base = 100u32;
    let production_shifted =
        segment_commands_with_offset_and_config(src, base, LexerConfig::default());

    let oracle_shifted: Vec<SegmentedCommand> = oracle(src, LexerConfig::default())
        .into_iter()
        .map(|c| c.shifted_by(base))
        .collect();

    assert_eq!(production_shifted.len(), oracle_shifted.len());
    for (p, o) in production_shifted.iter().zip(oracle_shifted.iter()) {
        assert_seg_eq(o, p, "nonzero-base");
    }
}

/// Recursively collect `.tcl` files under `dir`.
fn gather_tcl(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for entry in rd.flatten() {
        let p = entry.path();
        if p.is_dir() {
            gather_tcl(&p, out);
        } else if p.extension().is_some_and(|e| e == "tcl") {
            out.push(p);
        }
    }
}

#[test]
fn cst_matches_oracle_over_tcl_corpus() {
    let mut files = Vec::new();
    for version in ["tcl8.4.20", "tcl8.5.19", "tcl8.6.16", "tcl9.0.4"] {
        gather_tcl(&repo_root().join("tmp").join(version), &mut files);
    }
    if files.is_empty() {
        eprintln!("[differential_segment] skipped: no tmp/tcl* corpus present");
        return;
    }

    let mut checked = 0usize;
    let mut failures: Vec<String> = Vec::new();
    for path in &files {
        let Ok(src) = fs::read_to_string(path) else {
            continue;
        };
        checked += 1;
        let ctx = path.display().to_string();
        if let Err(msg) = check(&src, LexerConfig::default(), &ctx) {
            failures.push(msg);
            if failures.len() >= 20 {
                break;
            }
        }
    }
    assert!(checked > 0, "corpus present but no readable .tcl files");
    assert!(
        failures.is_empty(),
        "CST/oracle divergence over {checked} corpus files:\n{}",
        failures.join("\n")
    );
    eprintln!("[differential_segment] {checked} corpus files: CST == oracle");
}

/// Recovery is a post-process over `segment_commands_local`; confirm the
/// known-commands recovery path still works once the local segmenter is
/// CST-backed (only `segment_commands_local` is affected).
#[test]
fn recovery_known_commands_smoke() {
    use tcl_compiler::segmenter::segment_commands_with_recovery;
    let known: HashSet<&str> = ["puts", "set"].into_iter().collect();
    // An unclosed brace spanning several lines, then a recoverable `puts`.
    let src = "proc f {\n  body line\n  more\n  another\nputs recovered\n";
    let segs = segment_commands_with_recovery(src, &known);
    assert!(
        segs.iter().any(|s| s.is_partial),
        "expected a partial command from the unclosed brace"
    );
}

/// A frozen, byte-for-byte snapshot of the pre-CST token-loop segmenter.
///
/// Lifted verbatim from `rust/tcl-compiler/src/segmenter.rs` as it stood
/// before the token loop was replaced with the CST derivation.  Kept here
/// as the **independent reference** the CST port is differentially
/// validated against: comparing
/// the production segmenter (now CST-backed) against the live
/// `segment_commands_local` would be a tautology, so this frozen copy
/// preserves the original behaviour.  It deliberately duplicates
/// `command_span` / `widen_word_end` (rather than calling the crate's
/// `pub(crate)` versions) so the oracle is fully self-contained and cannot
/// drift with the production code.
///
/// Do not "fix" this module to match new behaviour — a divergence between
/// it and production is exactly what the harness exists to surface.
mod frozen_oracle {
    use tcl_lexer::{Lexer, LexerConfig, SourceMap, Span, Token, TokenType};

    use tcl_compiler::segmenter::{SegmentedCommand, WordFragment, word_piece};

    fn command_span(tokens: &[Token], sm: &SourceMap<'_>) -> Span {
        if tokens.is_empty() {
            return Span::new(0, 0);
        }
        let start = tokens.first().unwrap().span.start();
        let end = widen_word_end(*tokens.last().unwrap(), sm);
        Span::new(start, end)
    }

    fn widen_word_end(tok: Token, sm: &SourceMap<'_>) -> u32 {
        let closer = match tok.kind {
            TokenType::Str => b'}',
            TokenType::Cmd => b']',
            _ => return tok.span.end(),
        };
        let end = tok.span.end();
        if sm.token_text(tok).is_empty() {
            return end;
        }
        if sm.source().as_bytes().get(end as usize) == Some(&closer) {
            end + 1
        } else {
            end
        }
    }

    #[derive(Default)]
    struct SegmenterState {
        commands: Vec<SegmentedCommand>,
        argv: Vec<Token>,
        texts: Vec<String>,
        word_fragments: Vec<Vec<WordFragment>>,
        single: Vec<bool>,
        expand: Vec<bool>,
        all_tokens: Vec<Token>,
        has_expand: bool,
        last_comment: Option<String>,
    }

    impl SegmenterState {
        fn start_word(&mut self, tok: Token, piece: String, expanded: bool) {
            self.argv.push(tok);
            self.texts.push(piece.clone());
            self.word_fragments
                .push(vec![WordFragment::new(tok, piece)]);
            self.single.push(true);
            self.expand.push(expanded);
        }

        fn append_fragment(&mut self, tok: Token, piece: String) {
            if let Some(last_text) = self.texts.last_mut() {
                last_text.push_str(&piece);
            }
            if let Some(fragments) = self.word_fragments.last_mut() {
                fragments.push(WordFragment::new(tok, piece));
            }
            if let Some(single) = self.single.last_mut() {
                *single = false;
            }
            if let Some(previous) = self.argv.last_mut() {
                previous.span = Span::new(previous.span.start(), tok.span.end());
            }
        }

        fn flush_eol_or_eof(&mut self, tok: Token, sm: &SourceMap<'_>) {
            if !self.argv.is_empty() {
                self.commands.push(SegmentedCommand {
                    span: command_span(&self.all_tokens, sm),
                    argv: std::mem::take(&mut self.argv),
                    texts: std::mem::take(&mut self.texts),
                    word_fragments: std::mem::take(&mut self.word_fragments),
                    single_token_word: std::mem::take(&mut self.single),
                    all_tokens: std::mem::take(&mut self.all_tokens),
                    is_partial: false,
                    expand_word: if self.has_expand {
                        Some(std::mem::take(&mut self.expand))
                    } else {
                        self.expand.clear();
                        None
                    },
                    preceding_comment: self.last_comment.take(),
                    partial_delimiter: None,
                });
            } else if matches!(tok.kind, TokenType::Eol) {
                let eol_text = sm.token_text(tok);
                if eol_text.bytes().filter(|&b| b == b'\n').count() > 1 {
                    self.last_comment = None;
                }
            }
            self.has_expand = false;
        }
    }

    /// The frozen token loop, in local-offset space (`base_offset = 0`).
    pub fn segment_local(source: &str, config: LexerConfig) -> Vec<SegmentedCommand> {
        let sm = SourceMap::new(source);
        let config = LexerConfig {
            base_offset: 0,
            base_line: 0,
            base_col: 0,
            ..config
        };
        let lexer_sm = SourceMap::new(source);
        let lexer = Lexer::with_source_map(lexer_sm, config);
        let Ok(tokens) = lexer.tokenise_all() else {
            return Vec::new();
        };

        let mut state = SegmenterState::default();
        let mut prev_type = TokenType::Eol;
        let mut next_expand = false;

        for &tok in &tokens {
            match tok.kind {
                TokenType::Comment => {
                    let raw = sm.token_text(tok);
                    let line = raw.trim_start_matches('#').trim().to_string();
                    state.last_comment = Some(match state.last_comment.take() {
                        Some(prev) => format!("{prev}\n{line}"),
                        None => line,
                    });
                    continue;
                }
                TokenType::Sep => {
                    prev_type = tok.kind;
                    continue;
                }
                TokenType::Expand => {
                    state.all_tokens.push(tok);
                    next_expand = true;
                    state.has_expand = true;
                    prev_type = TokenType::Sep;
                    continue;
                }
                TokenType::Eol | TokenType::Eof => {
                    state.flush_eol_or_eof(tok, &sm);
                    next_expand = false;
                    prev_type = tok.kind;
                    continue;
                }
                _ => {}
            }

            state.all_tokens.push(tok);
            let piece = word_piece(&sm, tok);

            if prev_type == TokenType::Sep || prev_type == TokenType::Eol {
                state.start_word(tok, piece, next_expand);
                next_expand = false;
            } else if !state.texts.is_empty() {
                state.append_fragment(tok, piece);
            } else {
                state.start_word(tok, piece, next_expand);
                next_expand = false;
            }

            prev_type = tok.kind;
        }

        if state.argv.is_empty() {
            state.commands
        } else {
            let SegmenterState {
                mut commands,
                argv,
                texts,
                word_fragments,
                single,
                expand,
                all_tokens,
                has_expand,
                mut last_comment,
            } = state;
            commands.push(SegmentedCommand {
                span: command_span(&all_tokens, &sm),
                argv,
                texts,
                word_fragments,
                single_token_word: single,
                all_tokens,
                is_partial: false,
                expand_word: if has_expand { Some(expand) } else { None },
                preceding_comment: last_comment.take(),
                partial_delimiter: None,
            });
            commands
        }
    }
}
