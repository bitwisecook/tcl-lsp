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
//! [`WordExpr`] is built here from [`tcl_lexer::word_parts::ExecutablePartArena`]
//! — the one owner of "split a Tcl word into its substitution components" —
//! rather than from a private walk over lexer fragments. The
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
//!   leniency is a word-boundary rule, not a decomposition one. The word
//!   records C's message in [`WordExpr::Template`]'s `rejected`, for the
//!   consumer that must not run past a parse error.
//! - **Substitution boundaries** — where a `$` reference or `[…]` ends, which
//!   `$` is data, where C stops parsing — are the owner's, under the document's
//!   [`LexerConfig`] so the `${…}` close rule, the array-index source mask and
//!   the escape grammar follow the emulated release.
//! - **Spellings retain the actual reference.** A `Text` part carries its raw,
//!   undecoded source so the backslash rule stays explicit at this boundary; a
//!   `Variable` part carries its verbatim source spelling, including its scalar
//!   or element syntax. Compatibility argv text remains a separate view and
//!   cannot change the reference associated with a source read receipt. A
//!   `CommandSubstitution` carries `[script]`.
//! - **Wrapper spans include their original closing delimiter.** Expansion
//!   wrappers and templates use the shared token extent owner; empty groups
//!   already cover their closer and are never widened twice.
//! - **Component spans keep the lexer's inner-end convention.** A `${name}` or `[script]`
//!   part's span excludes its closer (an empty `${}` / `[]` covers it), which
//!   is what `codegen::wasm::leaf_invoke::plan_variable` reads to tell `$a(b)`
//!   from `${a(b)}`.
//! - **A parse error is the whole word.** C's `Tcl_ParseCommand` rejects the
//!   script, so the word becomes [`WordExpr::Opaque`] carrying C's message in
//!   [`WordOpacity::ParseError`] — the same `missing …` texts the segmenter's
//!   E200 reports — and every consumer declines it.

use tcl_lexer::word_parts::{ExecutablePart, ExecutablePartArena, SubstFlags, quoted_word_close};
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
        let end = original_run_end(sm.source(), *last, sm.base_offset(), config)
            .ok()
            .and_then(|end| u32::try_from(end).ok())
            .and_then(|end| sm.base_offset().checked_add(end))
            .unwrap_or(last.span.end());
        let word_span = Span::new(first.span.start(), end);
        // Sugar remains an opaque expression component with exact source.
        // The region builder retains quoted versus bare ownership separately.
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
    channel: tcl_lexer::SourceChannel,
}

/// Re-anchor a token's span into the buffer `sm` holds, so
/// [`SourceMap::token_text`] — which takes buffer-local spans — reads it.
fn localise(tok: Token, base: u32) -> Token {
    Token {
        span: Span::new(tok.span.start() - base, tok.span.end() - base),
        ..tok
    }
}

/// Keep the typed expression component's full written extent in the bare run.
fn original_run_end(
    source: &str,
    last: Token,
    base: u32,
    config: LexerConfig,
) -> Result<usize, &'static str> {
    let last_local = localise(last, base);
    if last.kind == TokenType::ExprSugar {
        Ok(tcl_lexer::word_parts::scan_expression_sugar(
            source.as_bytes(),
            last_local.span.start() as usize,
            config,
        )?
        .ok_or("missing original expression substitution")?
        .1)
    } else {
        let span = if last.kind == TokenType::Var {
            tcl_lexer::word_span_at(source, last_local.span)
        } else {
            tcl_lexer::word_span(&SourceMap::new(source), last_local)
        };
        Ok(span.end() as usize)
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
    let end = u32::try_from(original_run_end(source, last, base, config)?)
        .ok()
        .and_then(|end| base.checked_add(end))
        .ok_or("original word extent is out of range")?;
    let word_span = Span::new(first.span.start(), end);
    let opener_of = |tok: Token| bytes.get(at(tok.span.start())).copied();

    validate_braced_fragments(sm, fragments)?;

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
        channel: sm.channel(),
    };
    // Start of the bare run being accumulated, if one is open.
    let mut run_start: Option<usize> = None;
    // What C reports first for content welded to a closing brace or quote.
    let mut rejected: Option<&'static str> = None;
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
            if i < fragments.len() {
                rejected.get_or_insert(tcl_lexer::EXTRA_AFTER_CLOSE_BRACE);
            }
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
            if i < fragments.len() && config.quote_termination.is_strict() {
                rejected.get_or_insert(tcl_lexer::EXTRA_AFTER_CLOSE_QUOTE);
            }
            continue;
        }
        if run_start.is_none() {
            run_start = Some(start);
        }
        i += 1;
    }
    if let Some(start) = run_start {
        let end = original_run_end(source, last, base, config)?;
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
        [WordPart::Opaque { text, source }] if single_bare => WordExpr::Opaque {
            text: std::mem::take(text),
            source: source.clone(),
            reason: WordOpacity::DialectSubstitution,
        },
        _ => WordExpr::Template {
            parts: regions.parts,
            source: SourceSite::source(word_span),
            rejected,
        },
    };
    Ok(word)
}

fn validate_braced_fragments(sm: &SourceMap<'_>, fragments: &[Token]) -> Result<(), &'static str> {
    let source = sm.source();
    let base = sm.base_offset();
    let opener_of = |token: Token| {
        source
            .as_bytes()
            .get((token.span.start() - base) as usize)
            .copied()
    };
    // Recovery segmentation can retain an unterminated Str token without
    // marking the whole command partial. Its contents are diagnostic source,
    // never a proved literal value. Validate through the shared delimiter and
    // brace owners before either literal or compound-word promotion.
    for token in fragments
        .iter()
        .copied()
        .filter(|token| token.kind == TokenType::Str && opener_of(*token) == Some(b'{'))
    {
        let local = localise(token, base);
        let close = tcl_lexer::word_closer_offset_at(source, local.span)
            .ok_or(tcl_lexer::word_parts::MISSING_CLOSE_BRACE)?;
        let written = source
            .get(local.span.start() as usize..close as usize + 1)
            .ok_or(tcl_lexer::word_parts::MISSING_CLOSE_BRACE)?;
        if tcl_syntax::word_rules::whole_braced_word(written).is_none() {
            return Err(tcl_lexer::word_parts::MISSING_CLOSE_BRACE);
        }
    }

    Ok(())
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
    let content = source
        .get(start..end)
        .ok_or("component source unavailable")?;
    let length = u32::try_from(content.len()).map_err(|_| "component extent out of range")?;
    let image = tcl_lexer::SourceImage::from_bytes(content.as_bytes(), regions.channel);
    let arena =
        ExecutablePartArena::decompose(image, Span::new(0, length), SubstFlags::default(), config)
            .map_err(|_| "component source geometry unavailable")?;
    if let Some(message) = arena.all_parts().find_map(|part| match part.part {
        ExecutablePart::ParseError(message) => Some(message),
        _ => None,
    }) {
        return Err(message);
    }
    regions.count += 1;
    for component in arena.list(arena.root()) {
        let token_span = arena
            .source_span(component)
            .ok_or("component source extent unavailable")?;
        let offset = regions
            .base
            .checked_add(u32::try_from(start).map_err(|_| "component extent out of range")?)
            .ok_or("component extent out of range")?;
        let source_span = Span::new(
            offset
                .checked_add(token_span.start())
                .ok_or("component extent out of range")?,
            offset
                .checked_add(token_span.end())
                .ok_or("component extent out of range")?,
        );
        let raw = content
            .get(component.span.as_range())
            .ok_or("component source spelling unavailable")?;
        let source = SourceSite::source(source_span);
        regions.parts.push(match component.part {
            ExecutablePart::Text(_) => WordPart::Text {
                text: raw.to_owned(),
                source,
            },
            ExecutablePart::Variable { .. } => WordPart::Variable {
                spelling: raw.to_owned(),
                source,
            },
            ExecutablePart::Command { .. } => WordPart::CommandSubstitution {
                spelling: raw.to_owned(),
                source,
            },
            ExecutablePart::Expression { .. } => WordPart::Opaque {
                text: raw.to_owned(),
                source,
            },
            ExecutablePart::ParseError(message) => return Err(message),
        });
    }
    Ok(())
}

/// Compatibility argv projection only. Original source references and their
/// read receipts must retain their verbatim spelling instead of this view.
pub(crate) fn compatibility_variable_spelling(raw: &str) -> String {
    if raw.starts_with("${") {
        return raw.to_owned();
    }
    let Some(body) = raw.strip_prefix('$') else {
        return raw.to_owned();
    };
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

#[cfg(test)]
mod expression_source_tests {
    use super::*;

    fn original_word(source: &str) -> WordExpr {
        let config = LexerConfig::for_dialect("jim");
        let command =
            crate::segmenter::segment_commands_with_offset_and_config(source, 0, config).remove(0);
        let fragments = command.word_fragments[1]
            .iter()
            .map(|part| part.token)
            .collect::<Vec<_>>();
        WordExpr::from_word(
            &SourceMap::new(source),
            config,
            &fragments,
            &command.texts[1],
            false,
            None,
        )
    }

    #[test]
    fn jim_bare_and_mixed_sugar_keep_the_original_closing_parenthesis() {
        assert!(
            matches!(original_word("list $(k)"), WordExpr::Opaque { text, .. } if text == "$(k)")
        );
        let WordExpr::Template { parts, .. } = original_word("list pre$(1+(2))") else {
            panic!("mixed original word must remain a template");
        };
        assert!(
            matches!(&parts[1], WordPart::Opaque { text, source } if text == "$(1+(2))" && source.span == Span::new(8, 15))
        );
        assert!(
            matches!(original_word("list \"$(1+2)\""), WordExpr::Template { parts, .. } if matches!(&parts[..], [WordPart::Opaque { text, .. }] if text == "$(1+2)"))
        );
    }

    fn component_or_wrapper_extent(source: &str, word: &WordExpr) -> Span {
        match word {
            WordExpr::BracedLiteral { source: site, .. }
            | WordExpr::CommandSubstitution { source: site, .. } => {
                tcl_lexer::word_span_at(source, site.span)
            }
            _ => word.source().span,
        }
    }

    #[test]
    fn original_word_wrappers_keep_full_delimiters_without_trailing_source() {
        let config = LexerConfig::default();
        for (source, expected) in [
            ("list [change] ; # after", "[change]"),
            ("list [] ; # after", "[]"),
            ("list pre[change] ; # after", "pre[change]"),
            ("list pre[] ; # after", "pre[]"),
            ("list \"pre[change]\" ; # after", "\"pre[change]\""),
            ("list \"\" ; # after", "\"\""),
            ("list {value} ; # after", "{value}"),
            ("list {} ; # after", "{}"),
            ("list {value\\}} ; # after", "{value\\}}"),
            ("lappend x {*}[change] ; # after", "{*}[change]"),
            ("lappend x {*}[] ; # after", "{*}[]"),
            ("lappend x {*}\"value\" ; # after", "{*}\"value\""),
            ("lappend x {*}{value} ; # after", "{*}{value}"),
            ("lappend x {*}{} ; # after", "{*}{}"),
        ] {
            let command =
                crate::segmenter::segment_commands_with_offset_and_config(source, 0, config)
                    .remove(0);
            let words =
                crate::ir::CommandTokens::from_segmented(&SourceMap::new(source), config, &command);
            let word = words.word_exprs.last().unwrap();
            let extent = component_or_wrapper_extent(source, word);
            assert_eq!(&source[extent.as_range()], expected, "{source}");
            if let WordExpr::Expand { word, .. } = word {
                assert_eq!(
                    &source[component_or_wrapper_extent(source, word).as_range()],
                    expected.strip_prefix("{*}").unwrap(),
                    "{source}",
                );
            }
        }
    }

    #[test]
    fn expanded_word_extent_retains_derived_source_coordinates() {
        let source = "lappend x {*}[change]";
        let base = 71;
        let config = LexerConfig::default();
        let command =
            crate::segmenter::segment_commands_with_offset_and_config(source, base, config)
                .remove(0);
        let words = crate::ir::CommandTokens::from_segmented(
            &SourceMap::new(source).with_base(base, 0, 0),
            config,
            &command,
        );
        let word = words.word_exprs.last().unwrap();
        assert_eq!(word.source().span, Span::new(base + 10, base + 21));
        assert_eq!(
            &source[(word.source().span.start() - base) as usize
                ..(word.source().span.end() - base) as usize],
            "{*}[change]",
        );
    }
}
