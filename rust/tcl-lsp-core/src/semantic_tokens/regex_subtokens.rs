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

//! Sub-tokens inside regular expressions and `regsub` substitution specs.

use super::argument_override::{ArgOverride, subspec_content};
use super::emission::{Entry, TokenPositionContext, flush_run, push_subtoken};
use super::legend::TokenKind;
use rustc_hash::FxHashMap;
use tcl_lexer::{LineIndex, Token, TokenType};

/// Colour only the template roles wholly mapped to this original source fragment.
pub(super) fn push_template_subtokens(
    line_index: &LineIndex,
    source: &str,
    tok: Token,
    parts: &[(std::ops::Range<usize>, TokenKind)],
    literal: TokenKind,
    entries: &mut Vec<Entry>,
) -> bool {
    let Some((cstart, inner)) = subspec_content(source, tok) else {
        return false;
    };
    let pos = TokenPositionContext { source, line_index };
    let mut run = 0;
    for (span, kind) in parts {
        if span.start < cstart || span.end > cstart + inner.len() {
            continue;
        }
        let start = span.start - cstart;
        flush_run(pos, cstart, inner, run, start, literal, entries);
        push_subtoken(
            source,
            line_index,
            span.start,
            &source[span.clone()],
            *kind,
            entries,
        );
        run = span.end - cstart;
    }
    if run == 0 {
        return false;
    }
    flush_run(pos, cstart, inner, run, inner.len(), literal, entries);
    true
}

/// Tag each literal (`Str`/`Esc`) fragment of the word spanning `word_span`
/// with `ov`, leaving `Var`/`Cmd` substitution fragments untouched (they fall
/// through to the default classifier).  A single-fragment literal word (a
/// braced `{a+b}` pattern, a plain `"abc"`) is tagged as one piece — the
/// common case — while a word interleaving literals and substitutions gets
/// each literal run tagged independently.
pub(super) fn mark_literal_fragments(
    seg: &tcl_source::segment::SegmentedCommand,
    word_span: tcl_lexer::Span,
    ov: &ArgOverride,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    let coarse = ArgOverride::TemplateParts(TokenKind::Regexp, Vec::new());
    let ov = if matches!(ov, ArgOverride::RegexPattern)
        && !seg
            .argv
            .iter()
            .zip(&seg.single_token_word)
            .any(|(word, single)| word.span.start() == word_span.start() && *single)
    {
        // A missing fragment can turn later punctuation into class members,
        // a comment or literal text. Preserve only the known pattern role.
        &coarse
    } else {
        ov
    };
    for t in &seg.all_tokens {
        if t.span.start() >= word_span.start()
            && t.span.end() <= word_span.end()
            && matches!(t.kind, TokenType::Str | TokenType::Esc)
        {
            overrides
                .entry(t.span.start())
                .and_modify(|current| {
                    if matches!(current, ArgOverride::RegexPattern)
                        && matches!(ov, ArgOverride::TemplateParts(TokenKind::Regexp, _))
                    {
                        current.clone_from(ov);
                    }
                })
                .or_insert_with(|| ov.clone());
        }
    }
}

/// Retag the literal fragments of any word the compiler flagged as a regex
/// source (`set pat {…}` that later feeds `regexp`/`regsub`) so they highlight
/// as regex.  Keyed on the def-site word start; the compiler-supplied span is
/// authoritative for the fragment scan (the segmenter's token span can clamp a
/// closing delimiter).
pub(super) fn mark_regex_source_words(
    seg: &tcl_source::segment::SegmentedCommand,
    regex_sources: &FxHashMap<u32, tcl_lexer::Span>,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    if regex_sources.is_empty() {
        return;
    }
    for word in &seg.argv {
        if let Some(&full_span) = regex_sources.get(&word.span.start()) {
            mark_literal_fragments(seg, full_span, &ArgOverride::RegexPattern, overrides);
        }
    }
}

/// First-paint or definition-site display. Only a complete configured word
/// can supply regex components. Substituted words retain their Tcl tokens.
pub(super) fn push_regex_subtokens(
    ctx: super::ScriptCtx<'_>,
    tok: Token,
    entries: &mut Vec<Entry>,
) -> bool {
    let Some(word) = super::original::written_word(ctx, tok) else {
        return false;
    };
    let Some(value) = tcl_syntax::word_rules::original_static_word_ascii_presentation(&word) else {
        return false;
    };
    let Ok(text) = std::str::from_utf8(&value) else {
        return false;
    };
    let flags = ctx
        .original_words
        .and_then(|words| {
            words.with_source_schema(ctx.generation, crate::pattern_components::source_flags)
        })
        .flatten();
    let Some(roles) = crate::pattern_components::components(text, ctx.dialect, flags) else {
        return false;
    };
    let spans = original_component_spans(roles, |offset| {
        usize::try_from(
            tcl_syntax::word_rules::original_static_word_ascii_source_offset(&word, offset)?,
        )
        .ok()
    });
    push_template_subtokens(
        ctx.line_index,
        ctx.full_source,
        tok,
        &spans,
        TokenKind::Regexp,
        entries,
    )
}

/// Preserve each independently mapped component. A decoded value may have no
/// contiguous source spelling for an interior boundary.
pub(super) fn original_component_spans(
    roles: Vec<tcl_regex::PatternComponent>,
    mut offset: impl FnMut(usize) -> Option<usize>,
) -> Vec<(std::ops::Range<usize>, TokenKind)> {
    roles
        .into_iter()
        .filter_map(|item| {
            let range = offset(item.range.start)?..offset(item.range.end)?;
            (!range.is_empty()).then(|| (range, component_kind(item.role)))
        })
        .collect()
}

fn component_kind(role: tcl_regex::PatternRole) -> TokenKind {
    use tcl_regex::PatternRole as Role;
    match role {
        Role::Options
        | Role::CaptureOpen
        | Role::NoncaptureOpen
        | Role::PositiveLookahead
        | Role::NegativeLookahead
        | Role::GroupClose => TokenKind::RegexpGroup,
        Role::Comment => TokenKind::Comment,
        Role::Anchor => TokenKind::RegexpAnchor,
        Role::CharacterClass => TokenKind::RegexpCharClass,
        Role::Alternation => TokenKind::RegexpAlternation,
        Role::Quantifier => TokenKind::RegexpQuantifier,
        Role::Escape => TokenKind::RegexpEscape,
        Role::Backreference => TokenKind::RegexpBackref,
    }
}
