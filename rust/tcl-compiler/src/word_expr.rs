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

//! The compiler's adapter over the word-parts owner.
//!
//! [`WordExpr`] is built here from [`tcl_lexer::word_parts::decompose_spanned`]
//! — the one owner of "split a Tcl word into its substitution components" —
//! rather than from a private walk over lexer fragments (issue #1785). The
//! segmenter still owns *command* and *word* boundaries: it hands this module
//! one word's fragment tokens, and only the within-word breakdown is the
//! owner's.
//!
//! What stays lexical and what moves:
//!
//! - **Brace and quote extents** come from the lexer's fragments and
//!   [`tcl_lexer::quoted_word_close`]. A `{…}` fragment is a literal run with
//!   no substitution; a `"…"` run is decomposed as one region. Real Tcl rejects
//!   anything welded to either (`extra characters after close-brace`); the
//!   analyser accepts it so the braced part still gets diagnosed, and that
//!   leniency is a word-boundary rule (issue #1786), not a decomposition one.
//! - **Substitution boundaries** — where a `$` reference or `[…]` ends, which
//!   `$` is data, where C stops parsing — are the owner's, under the document's
//!   [`LexerConfig`] so the `${…}` close rule, the array-index source mask and
//!   the escape grammar follow the emulated release.
//! - **Spellings stay compatibility spellings.** A `Text` part carries its raw,
//!   undecoded source so the backslash rule stays explicit at this boundary; a
//!   `Variable` part carries the argv spelling the analyser and the WASM tiers
//!   read (`${name}` for a bare scalar, verbatim for `${…}` and for an element
//!   whose index substitutes); a `CommandSubstitution` carries `[script]`.
//! - **Spans keep the lexer's inner-end convention.** A `${name}` or `[script]`
//!   part's span excludes its closer (an empty `${}` / `[]` covers it), which
//!   is what `codegen::wasm::leaf_invoke::plan_variable` reads to tell `$a(b)`
//!   from `${a(b)}`.
//! - **A parse error is the whole word.** C's `Tcl_ParseCommand` rejects the
//!   script, so the word becomes [`WordExpr::Opaque`] carrying C's message in
//!   [`WordOpacity::ParseError`] — the same `missing …` texts the segmenter's
//!   E200 reports — and every consumer declines it.

use tcl_lexer::word_parts::{SpannedPart, SubstFlags, decompose_spanned, quoted_word_close};
use tcl_lexer::{LexerConfig, SourceMap, Span, Token, TokenType};

use crate::ir::{SourceSite, WordExpr, WordOpacity, WordPart};

impl WordExpr {
    /// Build the word model for one segmented word.
    ///
    /// `fragments` are the word's lexer tokens in document order (the
    /// segmenter's word boundary); `compat_text` is the argv spelling kept on
    /// an opaque word; `expansion_span` is the `{*}` marker when `expanded`.
    ///
    /// `sm` holds the buffer those spans index into. A sub-lex — the script
    /// inside a `[…]`, segmented at the offset it sits at — carries its
    /// document base on the map ([`SourceMap::with_base`]), so the text is
    /// read locally and every span this returns stays in the document's
    /// space.
    #[must_use]
    pub fn from_word(
        sm: &SourceMap<'_>,
        config: LexerConfig,
        fragments: &[Token],
        compat_text: &str,
        expanded: bool,
        expansion_span: Option<Span>,
    ) -> Self {
        let (Some(first), Some(last)) = (fragments.first(), fragments.last()) else {
            // No ordered fragments to decompose: the argv spelling is all
            // that survives, which is exactly what
            // [`WordOpacity::MissingFragments`] names.
            return Self::Opaque {
                text: compat_text.to_owned(),
                source: SourceSite::opaque(Span::new(0, 0)),
                reason: WordOpacity::MissingFragments,
            };
        };
        let word_span = Span::new(first.span.start(), last.span.end());
        // A dialect substitution the owner does not model must not reach the
        // literal promotion below: `decompose` reads `$(` as plain text, so a
        // JimTcl `$(1+2)` would become a `Literal` carrying its own spelling.
        if fragments
            .iter()
            .any(|t| t.kind == tcl_lexer::TokenType::ExprSugar)
        {
            let word = Self::Opaque {
                text: compat_text.to_owned(),
                source: SourceSite::opaque(word_span),
                reason: WordOpacity::DialectSubstitution,
            };
            return Self::maybe_expand(word, expanded, expansion_span, word_span);
        }
        let word = match build(sm, config, fragments) {
            Ok(word) => word,
            Err(message) => Self::Opaque {
                text: compat_text.to_owned(),
                source: SourceSite::opaque(word_span),
                reason: WordOpacity::ParseError(message),
            },
        };
        Self::maybe_expand(word, expanded, expansion_span, word_span)
    }

    /// Wrap `word` in [`WordExpr::Expand`] when the segmenter marked it with
    /// `{*}`, spanning from the marker so the expansion's own source is kept.
    fn maybe_expand(
        word: Self,
        expanded: bool,
        expansion_span: Option<Span>,
        word_span: Span,
    ) -> Self {
        if expanded {
            let start = expansion_span.map_or(word_span.start(), Span::start);
            Self::Expand {
                source: SourceSite::source(Span::new(start, word_span.end())),
                word: Box::new(word),
            }
        } else {
            word
        }
    }
}

/// The regions a word is made of: braced literals and quoted runs are
/// delimited by the lexer; everything between is a bare run.
struct Regions {
    parts: Vec<WordPart>,
    count: usize,
    quoted: bool,
    /// The document offset the decomposed buffer's first byte sits at, so a
    /// part built from a *local* slice offset carries its document span.
    base: u32,
}

/// Re-anchor a token's span into the buffer `sm` holds, so
/// [`SourceMap::token_text`] — which takes buffer-local spans — reads it.
fn localise(tok: Token, base: u32) -> Token {
    Token {
        span: Span::new(tok.span.start() - base, tok.span.end() - base),
        ..tok
    }
}

fn build(
    sm: &SourceMap<'_>,
    config: LexerConfig,
    fragments: &[Token],
) -> Result<WordExpr, &'static str> {
    let source = sm.source();
    let bytes = source.as_bytes();
    // The fragments' spans are in the *document's* space; `source` may be a
    // sub-lexed slice of it (a `[…]` substitution's script), so every index
    // into `source` drops the base and every span emitted keeps it.
    let base = sm.base_offset();
    let at = |offset: u32| (offset - base) as usize;
    let first = fragments[0];
    let last = fragments[fragments.len() - 1];
    let word_span = Span::new(first.span.start(), last.span.end());
    let opener_of = |tok: Token| bytes.get(at(tok.span.start())).copied();

    if fragments.len() == 1 && first.kind == TokenType::Str && opener_of(first) == Some(b'{') {
        return Ok(WordExpr::BracedLiteral {
            text: sm.token_text(localise(first, base)).to_owned(),
            source: SourceSite::source(first.span),
        });
    }

    let mut regions = Regions {
        parts: Vec::new(),
        count: 0,
        quoted: false,
        base,
    };
    // Start of the bare run being accumulated, if one is open.
    let mut run_start: Option<usize> = None;
    let mut i = 0;
    while i < fragments.len() {
        let tok = fragments[i];
        let start = at(tok.span.start());
        let opener = opener_of(tok);
        if tok.kind == TokenType::Str && opener == Some(b'{') {
            flush_run(&mut regions, source, config, run_start.take(), start)?;
            regions.parts.push(WordPart::Text {
                text: sm.token_text(localise(tok, base)).to_owned(),
                source: SourceSite::source(tok.span),
            });
            regions.count += 1;
            i += 1;
            continue;
        }
        if tok.kind == TokenType::Esc && tok.content_offset == 1 && opener == Some(b'"') {
            flush_run(&mut regions, source, config, run_start.take(), start)?;
            let close = quoted_word_close(source, start)?;
            decompose_region(&mut regions, source, config, start + 1, close)?;
            regions.quoted = true;
            let end = close + 1;
            // The rest of the quoted run's fragments — its `$` / `[` pieces
            // and the (possibly empty) closing-quote fragment — are covered.
            i += 1;
            while i < fragments.len() && at(fragments[i].span.end()) <= end {
                i += 1;
            }
            continue;
        }
        if run_start.is_none() {
            run_start = Some(start);
        }
        i += 1;
    }
    if let Some(start) = run_start {
        let end = tcl_lexer::word_span_at(source, localise(last, base).span).end() as usize;
        flush_run(&mut regions, source, config, Some(start), end)?;
    }

    let single_bare = regions.count == 1 && !regions.quoted;
    let word = match regions.parts.as_mut_slice() {
        [WordPart::Text { text, .. }] if single_bare && !text.contains('\\') => WordExpr::Literal {
            text: std::mem::take(text),
            source: SourceSite::source(word_span),
        },
        [WordPart::Variable { spelling, source }] if single_bare => WordExpr::Variable {
            spelling: std::mem::take(spelling),
            source: source.clone(),
        },
        [WordPart::CommandSubstitution { spelling, source }] if single_bare => {
            WordExpr::CommandSubstitution {
                spelling: std::mem::take(spelling),
                source: source.clone(),
            }
        }
        _ => WordExpr::Template {
            parts: regions.parts,
            source: SourceSite::source(word_span),
        },
    };
    Ok(word)
}

/// Close the bare run `[start, end)`, if one is open, as one decomposed
/// region. Both offsets are local to `source`.
fn flush_run(
    regions: &mut Regions,
    source: &str,
    config: LexerConfig,
    start: Option<usize>,
    end: usize,
) -> Result<(), &'static str> {
    match start {
        Some(start) if end > start => decompose_region(regions, source, config, start, end),
        _ => Ok(()),
    }
}

/// Decompose `source[start..end]` through the owner and append its parts.
fn decompose_region(
    regions: &mut Regions,
    source: &str,
    config: LexerConfig,
    start: usize,
    end: usize,
) -> Result<(), &'static str> {
    let content = source.get(start..end).unwrap_or("");
    regions.count += 1;
    let base = regions.base;
    for spanned in decompose_spanned(content.as_bytes(), SubstFlags::default(), config) {
        regions.parts.push(part_at(source, start, base, &spanned)?);
    }
    Ok(())
}

/// One owner part, re-anchored at the local offset `start` in `source` and
/// carried out in the document space `base` anchors.
fn part_at(
    source: &str,
    start: usize,
    base: u32,
    spanned: &SpannedPart<'_>,
) -> Result<WordPart, &'static str> {
    let (start, end) = (start + spanned.start, start + spanned.end);
    let raw = source.get(start..end).unwrap_or("");
    let span = |end: usize| Span::new(offset(start) + base, offset(end) + base);
    Ok(match &spanned.part {
        tcl_lexer::WordPart::Text(_) => WordPart::Text {
            text: raw.to_owned(),
            source: SourceSite::source(span(end)),
        },
        tcl_lexer::WordPart::Variable(_) => {
            // The lexer's inner-end convention: a `${name}` token stops short
            // of its closing brace unless the name is empty.
            let braced = raw.starts_with("${");
            let inner_end = if braced && raw.len() > 3 {
                end - 1
            } else {
                end
            };
            WordPart::Variable {
                spelling: variable_spelling(raw),
                source: SourceSite::source(span(inner_end)),
            }
        }
        tcl_lexer::WordPart::Command(_) => {
            let inner_end = if raw.len() > 2 { end - 1 } else { end };
            WordPart::CommandSubstitution {
                spelling: raw.to_owned(),
                source: SourceSite::source(span(inner_end)),
            }
        }
        tcl_lexer::WordPart::ParseError(message) => return Err(message),
    })
}

/// The compatibility argv spelling of the variable reference written as `raw`.
///
/// A `${…}` reference and a bare element whose index itself substitutes
/// (`$arr($i)`) round-trip verbatim — wrapping the latter in braces would turn
/// the element access into a scalar lookup of a name with parentheses in it.
/// A bare scalar or literal-index element is normalised to `${name}` so
/// consumers read one canonical shape; a name containing `}` cannot be
/// braced unambiguously and stays bare.
fn variable_spelling(raw: &str) -> String {
    if raw.starts_with("${") {
        return raw.to_owned();
    }
    let body = &raw[1..];
    if let Some(open) = body.find('(')
        && body.ends_with(')')
        && body[open..].contains(['$', '['])
    {
        return raw.to_owned();
    }
    if body.contains('}') {
        raw.to_owned()
    } else {
        format!("${{{body}}}")
    }
}

fn offset(at: usize) -> u32 {
    u32::try_from(at).expect("source offsets fit in u32")
}
