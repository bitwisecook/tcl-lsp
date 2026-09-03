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

//! Differential harness: owner-based `WordExpr` vs the fragment-walk oracle.
//!
//! `CommandTokens::from_segmented` — the builder every production lowering
//! calls — builds every word's `WordExpr` from
//! `tcl_lexer::word_parts::decompose_spanned` (issue #1785). Before that it
//! mapped the lexer's fragment tokens one-for-one, and that walk is frozen
//! here as the **independent oracle** so the two can be compared forever:
//! over a crafted edge-case table under both release axes, over `samples/`,
//! and over `tmp/tcllib-2.0` when it is present.
//!
//! Every comparison runs the *shipping* builder and, beside it, the owner
//! invoked directly: they must agree word-for-word, so this harness cannot
//! go green on a production path that quietly stopped going through the
//! owner (the gap a reviewer caught when `from_word` shipped dead).
//!
//! The comparison is exact up to the oracle's enumerated artefacts —
//! [`canonical`] documents each one — so a new divergence is a real
//! behavioural change, not noise.

use std::fs;
use std::path::{Path, PathBuf};

use tcl_lexer::{LexerConfig, SourceMap, Span, Token, TokenType};

use tcl_compiler::ir::{CommandTokens, SourceSite, WordExpr, WordOpacity, WordPart};
use tcl_compiler::segmenter::{
    SegmentedCommand, WordFragment, segment_commands_with_offset_and_config,
};

/// Repo root — two directories above `CARGO_MANIFEST_DIR`.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn nine() -> LexerConfig {
    LexerConfig::default()
}

fn eight() -> LexerConfig {
    LexerConfig::for_dialect("tcl8.4")
}

fn segments(src: &str, config: LexerConfig) -> Vec<SegmentedCommand> {
    segment_commands_with_offset_and_config(src, 0, config)
}

/// The **production** builder under test: the shipping `from_segmented`,
/// which is what every lowering, the WASM leaf-invoke planner and the native
/// lowerer actually call.
fn production(src: &str, config: LexerConfig, seg: &SegmentedCommand) -> Vec<WordExpr> {
    let sm = SourceMap::new(src);
    CommandTokens::from_segmented(&sm, config, seg).word_exprs
}

/// The owner-based builder invoked directly, with the segmenter's word
/// boundaries — the shape [`production`] is asserted to reproduce, so a
/// production path that stopped going through the owner fails here rather
/// than passing on the strength of this function alone.
fn owner_based(src: &str, config: LexerConfig, seg: &SegmentedCommand) -> Vec<WordExpr> {
    let sm = SourceMap::new(src);
    let expand = seg.expand_word.as_deref();
    seg.argv
        .iter()
        .enumerate()
        .map(|(idx, token)| {
            let fragments: Vec<Token> = seg
                .word_fragments
                .get(idx)
                .map_or(&[][..], Vec::as_slice)
                .iter()
                .map(|f| f.token)
                .collect();
            let text = seg.texts.get(idx).map_or("", String::as_str);
            let expanded = expand
                .and_then(|flags| flags.get(idx))
                .copied()
                .unwrap_or(false);
            let expansion_span = expanded
                .then(|| {
                    seg.all_tokens
                        .iter()
                        .rev()
                        .find(|candidate| {
                            candidate.kind == TokenType::Expand
                                && candidate.span.end() <= token.span.start()
                        })
                        .map(|candidate| candidate.span)
                })
                .flatten();
            WordExpr::from_word(&sm, config, &fragments, text, expanded, expansion_span)
        })
        .collect()
}

/// The **oracle**: the frozen fragment walk (see [`frozen_oracle`]).
fn oracle(seg: &SegmentedCommand) -> Vec<WordExpr> {
    frozen_oracle::word_exprs(seg)
}

/// The oracle's artefacts, normalised away so the comparison is exact:
///
/// 1. **Empty text parts at a quote.** The lexer emits an empty `Esc` for the
///    opening `"` when the body starts with `$` / `[`, and one for the closing
///    `"`; the oracle kept them as empty `Text` parts. They are dropped unless
///    the word is exactly `""`, whose one empty part is its value.
/// 2. **The opening quote inside the first text span.** The first `Esc` of a
///    quoted run has `content_offset = 1`, so its span covers the `"`; the
///    owner's text extents are content only.
/// 3. **A name-less `$` as its own part.** The lexer emits a `$` with no name
///    behind it as a one-byte `Str`, so the oracle split `price$` into two
///    text parts (or made a lone `$` a `BracedLiteral`). C treats that `$` as
///    data in the surrounding run, which is what the owner reports.
/// 4. **A bare word of one text run collapses to `Literal`** when it has no
///    backslash — the shape the production builder reports once artefact 3
///    is folded.
/// 5. **Unterminated words.** The lexer tokenises an unclosed `"…`, `[…`,
///    `${…` and `$arr(` best-effort and the oracle modelled them as ordinary
///    parts; the owner reports C's parse error and the word is `Opaque`.
///    The oracle side is rewritten to the same `Opaque` from the message
///    the owner names, so a *wrong* message still fails.
fn canonical(word: WordExpr, source: &str, production: &WordExpr) -> WordExpr {
    if let WordExpr::Opaque {
        reason: WordOpacity::ParseError(_),
        ..
    } = production
        && !matches!(word, WordExpr::Opaque { .. })
    {
        return production.clone();
    }
    match word {
        WordExpr::Expand { source: site, word } => WordExpr::Expand {
            source: site,
            word: Box::new(canonical(*word, source, expand_inner(production))),
        },
        WordExpr::BracedLiteral { text, source: site }
            if text == "$" && at(source, &site) == "$" =>
        {
            WordExpr::Literal { text, source: site }
        }
        WordExpr::Template {
            parts,
            source: site,
        } => canonical_template(parts, site, source),
        other => other,
    }
}

fn expand_inner(word: &WordExpr) -> &WordExpr {
    match word {
        WordExpr::Expand { word, .. } => word,
        other => other,
    }
}

fn at<'s>(source: &'s str, site: &SourceSite) -> &'s str {
    source
        .get(site.span.start() as usize..site.span.end() as usize)
        .unwrap_or("")
}

fn canonical_template(parts: Vec<WordPart>, site: SourceSite, source: &str) -> WordExpr {
    // Whether the *word* is a quoted one — the `""` exception below and
    // artefact 4 are word-level rules, unlike the per-run fold in the loop.
    let quoted = source.as_bytes().get(site.span.start() as usize) == Some(&b'"');
    let mut out: Vec<WordPart> = Vec::with_capacity(parts.len());
    for part in parts {
        let WordPart::Text {
            text,
            source: mut part_site,
        } = part
        else {
            out.push(part);
            continue;
        };
        // Artefact 2. The fold is a property of the *quoted run*, not of the
        // word: `foo {*}"a $b"` opens its quote at 7 while the word starts at
        // the `{*}` marker, so anchoring this on the word start missed it. A
        // text part whose span opens on a `"` the text itself does not carry
        // is the oracle's `content_offset = 1` fold; a genuine `"` of content
        // (inside `{"a"}`) starts the text too and is left alone.
        if source.as_bytes().get(part_site.span.start() as usize) == Some(&b'"')
            && !text.starts_with('"')
        {
            part_site = SourceSite::source(Span::new(
                part_site.span.start() + 1,
                part_site.span.end().max(part_site.span.start() + 1),
            ));
        }
        // Artefact 1 (the `""` exception is restored below).
        if text.is_empty() {
            continue;
        }
        // Artefact 3: a `$` run welds onto its neighbours.
        let welds =
            text == "$" || matches!(out.last(), Some(WordPart::Text { text, .. }) if text == "$");
        if welds
            && let Some(WordPart::Text {
                text: prev,
                source: prev_site,
            }) = out.last_mut()
            && prev_site.span.end() == part_site.span.start()
        {
            prev.push_str(&text);
            *prev_site =
                SourceSite::source(Span::new(prev_site.span.start(), part_site.span.end()));
            continue;
        }
        out.push(WordPart::Text {
            text,
            source: part_site,
        });
    }
    if out.is_empty() && quoted {
        let inner = Span::new(site.span.start() + 1, site.span.start() + 1);
        out.push(WordPart::Text {
            text: String::new(),
            source: SourceSite::source(inner),
        });
    }
    // Artefact 4.
    if !quoted
        && let [WordPart::Text { text, .. }] = out.as_slice()
        && !text.contains('\\')
        && at(source, &site) == text
    {
        return WordExpr::Literal {
            text: text.clone(),
            source: site,
        };
    }
    WordExpr::Template {
        parts: out,
        source: site,
    }
}

fn check(src: &str, config: LexerConfig, ctx: &str) -> Result<usize, String> {
    let mut words = 0usize;
    for (ci, seg) in segments(src, config).iter().enumerate() {
        // The shipping path is what is asserted; the direct owner call is
        // held beside it so a divergence names which side moved.
        let got = production(src, config, seg);
        let owner = owner_based(src, config, seg);
        if got != owner {
            return Err(format!(
                "[{ctx}] cmd {ci}: production does not go through the owner:\n  \
                 production: {got:#?}\n  owner:      {owner:#?}"
            ));
        }
        let want = oracle(seg);
        if got.len() != want.len() {
            return Err(format!(
                "[{ctx}] cmd {ci}: word count {} vs oracle {}",
                got.len(),
                want.len()
            ));
        }
        for (wi, (g, o)) in got.iter().zip(want).enumerate() {
            words += 1;
            let want = canonical(o, src, g);
            if *g != want {
                let raw = at(src, g.source());
                return Err(format!(
                    "[{ctx}] cmd {ci} word {wi} {raw:?}:\n  production: {g:#?}\n  oracle:     {want:#?}"
                ));
            }
        }
    }
    Ok(words)
}

/// Crafted words covering every part kind, both spellings of a reference,
/// every delimiter welded to another, and each of C's parse errors.
const EDGE_CASES: &[&str] = &[
    "",
    "puts hi",
    "set x 1\nputs $x\n",
    "puts $x",
    "puts ${x}",
    "puts ${}",
    "puts $arr(k)",
    "puts $arr($i)",
    "puts $arr([k])",
    "puts $arr(a,$b)",
    "puts $a:::b",
    "puts $::ns::v",
    "puts $",
    "puts price$",
    "puts price$ x",
    "puts a$b",
    "puts a${b}c",
    "puts a[b]c",
    "puts [b]",
    "puts []",
    "puts [a [b] c]",
    "puts [list {a]b}]",
    "puts [list \"a]b\"]",
    "puts a\\nb",
    "puts a\\$b",
    "puts \\$a",
    "puts \\[a]",
    "puts a\\",
    "puts \"\"",
    "puts \"abc\"",
    "puts \"a $b c\"",
    "puts \"$a\"",
    "puts \"$a [b] $c\"",
    "puts \"[b]\"",
    "puts \"a\\\"b\"",
    "puts \"a[foo \"b\"]c\"",
    "puts \"a\\\nb\"",
    "puts \"\\\n\"",
    "puts \"ab\"\"cd\"",
    "puts \"a\"b",
    "puts \"a\"$b",
    "puts a\"b\"c",
    "puts {a}",
    "puts {}",
    "puts {a}b",
    "puts {a}$b",
    "puts {a}{b}",
    "puts a{b}c",
    "puts {a\\}b}",
    "puts {a $b [c]}",
    "foo {*}$args",
    "foo {*}{*}$args",
    "foo {*}",
    "foo {*}[cmd]",
    "foo {*}{a b}",
    "foo {*}\"a $b\"",
    "foo {*}a$b",
    "list a \\\n b",
    "puts ${a{b}c}",
    "puts ${a\\}b}",
    "puts ${abc",
    "puts ${a{b}",
    "puts $arr({k})",
    "puts $arr(",
    "puts x[b",
    "puts [side][b",
    "puts \"abc",
    "puts \"a $b",
    "puts {abc",
    "puts [set y ${a{b]",
    "puts \\x4142",
    "puts \\U0001F600",
    "puts \"\\x41$b\"",
    "if {$x} {body}",
    "proc f {a b} { return [expr {$a + $b}] }",
    "set s \"prefix-$name-[clock seconds]\"",
    "puts $a($b($c(1)))",
    "puts a;puts b",
    "# comment\nputs hi",
    "puts é$x",
    "puts \"日本 $x\"",
];

#[test]
fn owner_matches_oracle_on_edge_cases() {
    let mut failures = Vec::new();
    for (idx, src) in EDGE_CASES.iter().enumerate() {
        for (label, config) in [("9.x", nine()), ("8.x", eight())] {
            if let Err(msg) = check(src, config, &format!("edge {idx} {src:?} {label}")) {
                failures.push(msg);
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

/// Recursively collect Tcl sources under `dir`.
fn gather(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for entry in rd.flatten() {
        let p = entry.path();
        if p.is_dir() {
            gather(&p, out);
        } else if p
            .extension()
            .is_some_and(|e| e == "tcl" || e == "irul" || e == "irule")
        {
            out.push(p);
        }
    }
}

fn sweep(files: &[PathBuf], label: &str) {
    let mut checked = 0usize;
    let mut words = 0usize;
    let mut failures: Vec<String> = Vec::new();
    for path in files {
        let Ok(src) = fs::read_to_string(path) else {
            continue;
        };
        checked += 1;
        let ctx = path.display().to_string();
        match check(&src, nine(), &ctx) {
            Ok(n) => words += n,
            Err(msg) => {
                failures.push(msg);
                if failures.len() >= 20 {
                    break;
                }
            }
        }
    }
    assert!(checked > 0, "{label}: no readable sources");
    assert!(
        failures.is_empty(),
        "owner/oracle divergence over {checked} {label} files:\n{}",
        failures.join("\n\n")
    );
    eprintln!("[differential_word_expr] {label}: {checked} files, {words} words agree");
}

#[test]
fn owner_matches_oracle_over_samples() {
    let mut files = Vec::new();
    gather(&repo_root().join("samples"), &mut files);
    sweep(&files, "samples");
}

#[test]
fn owner_matches_oracle_over_tcllib() {
    let mut files = Vec::new();
    gather(&repo_root().join("tmp").join("tcllib-2.0"), &mut files);
    if files.is_empty() {
        eprintln!("[differential_word_expr] skipped: no tmp/tcllib-2.0 corpus present");
        return;
    }
    sweep(&files, "tcllib-2.0");
}

/// A parse error the owner names reaches the word as a typed opaque word
/// carrying C's message, and the oracle side cannot fake that.
#[test]
fn parse_errors_carry_c_tcls_message() {
    let cases = [
        ("puts x[b", "missing close-bracket"),
        ("puts $arr(", "missing )"),
        ("puts ${abc", "missing close-brace for variable name"),
        ("puts \"abc", "missing \""),
        (
            "puts [set y ${a{b]",
            "missing close-brace for variable name",
        ),
    ];
    for (src, message) in cases {
        let segs = segments(src, nine());
        let words = production(src, nine(), &segs[0]);
        assert!(
            matches!(
                &words[1],
                WordExpr::Opaque {
                    reason: WordOpacity::ParseError(m),
                    ..
                } if *m == message
            ),
            "{src:?}: {:?}",
            words[1]
        );
    }
    // Tcl 9 rejects a raw brace in an array index; Tcl 8 passes it through.
    let words = production(
        "puts $arr({k})",
        nine(),
        &segments("puts $arr({k})", nine())[0],
    );
    assert!(
        matches!(
            &words[1],
            WordExpr::Opaque {
                reason: WordOpacity::ParseError("invalid character in array index"),
                ..
            }
        ),
        "{:?}",
        words[1]
    );
    // Tcl 8 keeps the reference, and its compatibility spelling stays the raw
    // source rather than the `${…}` wrap every other scalar gets. Measured on
    // tclsh 8.6.16: `set arr({k}) hello; set v $arr({k})` yields `hello`,
    // while `${arr({k})}` reads the *name* `arr({k` under the release's
    // first-close rule (`can't read "arr({k": no such variable`). So the wrap
    // is only sound while the body carries no `}` — which is exactly the
    // guard `variable_spelling` applies. (On tclsh 9.0.4 the brace form does
    // resolve, its close rule being nesting-aware, but the bare form there is
    // the parse error asserted above, so this arm is 8.x-only anyway.)
    let words = production(
        "puts $arr({k})",
        eight(),
        &segments("puts $arr({k})", eight())[0],
    );
    assert!(
        matches!(&words[1], WordExpr::Variable { spelling, .. } if spelling == "$arr({k})"),
        "{:?}",
        words[1]
    );
}

/// The frozen fragment walk: a byte-for-byte copy of the pre-#1785
/// `WordExpr::from_fragments` / `WordPart::from_fragment` /
/// `is_plain_bare_literal`, mapping each lexer fragment token to one part.
mod frozen_oracle {
    use super::*;

    pub fn word_exprs(seg: &SegmentedCommand) -> Vec<WordExpr> {
        let expand = seg.expand_word.as_deref();
        seg.argv
            .iter()
            .enumerate()
            .map(|(idx, token)| {
                let fragments = seg.word_fragments.get(idx).map_or(&[][..], Vec::as_slice);
                let text = seg.texts.get(idx).map_or("", String::as_str);
                let expanded = expand
                    .and_then(|flags| flags.get(idx))
                    .copied()
                    .unwrap_or(false);
                let expansion_span = expanded
                    .then(|| {
                        seg.all_tokens
                            .iter()
                            .rev()
                            .find(|candidate| {
                                candidate.kind == TokenType::Expand
                                    && candidate.span.end() <= token.span.start()
                            })
                            .map(|candidate| candidate.span)
                    })
                    .flatten();
                from_fragments(fragments, text, token.span, expanded, expansion_span)
            })
            .collect()
    }

    fn from_fragments(
        fragments: &[WordFragment],
        fallback_text: &str,
        fallback_span: Span,
        expanded: bool,
        expansion_span: Option<Span>,
    ) -> WordExpr {
        let word = match fragments {
            [fragment] if is_plain_bare_literal(fragment) => WordExpr::Literal {
                text: fragment.text.clone(),
                source: SourceSite::source(fragment.token.span),
            },
            [fragment] if fragment.token.kind == TokenType::Str => WordExpr::BracedLiteral {
                text: fragment.text.clone(),
                source: SourceSite::source(fragment.token.span),
            },
            [fragment] if fragment.token.kind == TokenType::Var => WordExpr::Variable {
                spelling: fragment.text.clone(),
                source: SourceSite::source(fragment.token.span),
            },
            [fragment] if fragment.token.kind == TokenType::Cmd => WordExpr::CommandSubstitution {
                spelling: fragment.text.clone(),
                source: SourceSite::source(fragment.token.span),
            },
            [] => WordExpr::Opaque {
                text: fallback_text.to_owned(),
                source: SourceSite::opaque(fallback_span),
                reason: WordOpacity::MissingFragments,
            },
            _ => WordExpr::Template {
                parts: fragments.iter().map(from_fragment).collect(),
                source: SourceSite::source(fallback_span),
            },
        };
        if expanded {
            let start = expansion_span.map_or(fallback_span.start(), Span::start);
            WordExpr::Expand {
                source: SourceSite::source(Span::new(start, fallback_span.end())),
                word: Box::new(word),
            }
        } else {
            word
        }
    }

    fn is_plain_bare_literal(fragment: &WordFragment) -> bool {
        fragment.token.kind == TokenType::Esc
            && !fragment.token.in_quote
            && fragment.token.content_offset == 0
            && !fragment.text.contains('\\')
    }

    fn from_fragment(fragment: &WordFragment) -> WordPart {
        let source = SourceSite::source(fragment.token.span);
        match fragment.token.kind {
            TokenType::Var => WordPart::Variable {
                spelling: fragment.text.clone(),
                source,
            },
            TokenType::Cmd => WordPart::CommandSubstitution {
                spelling: fragment.text.clone(),
                source,
            },
            TokenType::Esc | TokenType::Str => WordPart::Text {
                text: fragment.text.clone(),
                source,
            },
            _ => WordPart::Opaque {
                text: fragment.text.clone(),
                source,
            },
        }
    }
}
