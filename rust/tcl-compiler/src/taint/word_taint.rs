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

//! The taint of one argument word: variable reads, interpolation over the original source image and the structural carve-outs.

use super::definition_taint::var_taint;
use super::sink_warnings::static_word_expr_value;
use super::substitution_taint::{retained_taint_substitution, substitution_word_taint};
use super::{TaintColour, TaintCtx, TaintLattice, TaintProvenance};
use crate::depth_guard::MAX_BRACKET_TEXT_DEPTH;
use crate::ssa::{Symbol, ValueKey};
use crate::value_shapes::parse_command_substitution_with_config;
use std::collections::HashMap;
use tcl_lexer::{Lexer, SourceMap, Span, TokenType, backslash_subst_in};

/// Infer the taint of an argument word from already-known per-variable
/// taint values.
///
/// Handles pure variable references (`$x`), bracketed command
/// substitutions (`[cmd ...]`), and interpolated strings.
pub(crate) fn word_taint<S: std::hash::BuildHasher>(
    word: &str,
    uses: &HashMap<Symbol, u32>,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    ctx: TaintCtx<'_>,
) -> TaintLattice {
    // Public entry: a word's raw text is bracket-nesting depth 0; the
    // recursion cap lives in [`word_taint_at`].
    word_taint_at(word, uses, taints, ctx, 0)
}

/// Depth-carrying core of [`word_taint`]. `depth` counts nested `[cmd …]`
/// command substitutions *within this word's raw text* — a genuinely
/// unbounded recursion axis, independent of any expression- or
/// statement-tree cap. `[a [b [c …]]]` nested N deep recurses N frames here.
pub(super) fn word_taint_at<S: std::hash::BuildHasher>(
    word: &str,
    uses: &HashMap<Symbol, u32>,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    ctx: TaintCtx<'_>,
    depth: u32,
) -> TaintLattice {
    // Native-stack safety net. Past the cap, assume the word is tainted with
    // no proven mitigations — the conservative direction for a taint
    // analysis, so a source buried in nesting we stopped scanning can never
    // be laundered into a false-negative security result.
    if MAX_BRACKET_TEXT_DEPTH.exceeded(depth) {
        return TaintLattice::tainted();
    }
    // The word's shape is decided in a leaf, so its parse temporaries are not
    // held on every level of nested substitution.
    match word_taint_step(word, ctx) {
        WordTaintStep::Done(taint) => taint,
        WordTaintStep::Substitution(invocation) => {
            let (command, args) = &*invocation;
            substitution_word_taint(word, (command, args), uses, taints, ctx, depth)
        }
        WordTaintStep::SourceInterpolation => {
            source_interpolation_taint(word, uses, taints, ctx, depth)
                .unwrap_or_else(TaintLattice::tainted)
        }
        WordTaintStep::StandaloneInterpolation => {
            standalone_interpolation_taint(word, word.trim(), uses, taints, ctx, depth)
        }
    }
}

/// How [`word_taint_at`] continues with one word.
enum WordTaintStep {
    Done(TaintLattice),
    Substitution(Box<(String, Vec<String>)>),
    SourceInterpolation,
    StandaloneInterpolation,
}

#[inline(never)]
fn word_taint_step(word: &str, ctx: TaintCtx<'_>) -> WordTaintStep {
    if ctx.source_word.is_some_and(|source_word| {
        static_word_expr_value(
            source_word,
            ctx.lexer_config().escapes,
            tcl_syntax::word_rules::WordValueRules::from_config(&ctx.lexer_config()),
        )
        .is_some()
    }) {
        return WordTaintStep::Done(TaintLattice::clean());
    }
    let stripped = word.trim();

    // A source word's structured form selects the actual command component;
    // its compatibility text cannot invent a whole-word substitution.
    if let Some(source_word) = ctx.source_word {
        if let Some(nested) =
            crate::word_subst::whole_word_command_tokens(source_word, ctx.lexer_config())
        {
            let Some(command) = nested.argv_texts.first() else {
                return WordTaintStep::Done(TaintLattice::tainted());
            };
            let args = nested
                .argv_texts
                .iter()
                .skip(1)
                .cloned()
                .collect::<Vec<_>>();
            return WordTaintStep::Substitution(Box::new((command.clone(), args)));
        }
    } else if ctx.source.allows_nominal_metadata()
        && let Some(invocation) =
            parse_command_substitution_with_config(stripped, ctx.lexer_config())
    {
        return WordTaintStep::Substitution(Box::new(invocation));
    }

    if !ctx.source.allows_nominal_metadata() {
        return WordTaintStep::SourceInterpolation;
    }
    if stripped.contains('$') || stripped.contains('[') {
        return WordTaintStep::StandaloneInterpolation;
    }
    WordTaintStep::Done(TaintLattice::clean())
}

/// Interpolation taint from one retained original image and source word.
fn source_interpolation_taint<S: std::hash::BuildHasher>(
    word: &str,
    uses: &HashMap<Symbol, u32>,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    ctx: TaintCtx<'_>,
    depth: u32,
) -> Option<TaintLattice> {
    let (image, span) = retained_word_source(word, ctx)?;
    let Some(source_text) = image
        .bytes()
        .get(span.as_range())
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
        .map(str::to_owned)
    else {
        return Some(TaintLattice::tainted());
    };
    let Ok(arena) = tcl_lexer::ExecutablePartArena::decompose(
        image,
        span,
        tcl_lexer::SubstFlags::default(),
        ctx.lexer_config(),
    ) else {
        return Some(TaintLattice::tainted());
    };
    let mut taint = TaintLattice::clean();
    let mut sole_read = false;
    for part in arena.all_parts() {
        match part.part {
            tcl_lexer::ExecutablePart::Variable { .. } => {
                sole_read = part.span == span;
                let Some(spelling) = arena
                    .bytes(part.span)
                    .and_then(|bytes| std::str::from_utf8(bytes).ok())
                else {
                    return Some(TaintLattice::tainted());
                };
                let valid = tcl_syntax::naming::variable_reference_root_bytes(
                    spelling.as_bytes(),
                    ctx.lexer_config(),
                )
                .ok()
                .flatten()
                .and_then(|name| std::str::from_utf8(name).ok())
                .is_some_and(|name| !name.contains('\0'));
                if !valid {
                    return Some(TaintLattice::tainted());
                }
                taint = taint.join(original_component_read_taint(
                    ctx, part.span, spelling, taints,
                ));
            }
            tcl_lexer::ExecutablePart::Command { .. } => {
                let Some(sub) = arena
                    .bytes(part.span)
                    .and_then(|bytes| std::str::from_utf8(bytes).ok())
                else {
                    return Some(TaintLattice::tainted());
                };
                let sub_ctx = TaintCtx {
                    source_word: None,
                    word_offset: Some(part.span.start()),
                    ..ctx
                };
                taint = taint.join(retained_command_part_taint(
                    sub,
                    uses,
                    taints,
                    sub_ctx,
                    depth + 1,
                ));
            }
            tcl_lexer::ExecutablePart::Expression { .. }
            | tcl_lexer::ExecutablePart::ParseError(_) => {
                return Some(TaintLattice::tainted());
            }
            tcl_lexer::ExecutablePart::Text(_) => {}
        }
    }
    if sole_read {
        // A word that is one variable read copies the value, colours and all.
        return Some(taint);
    }
    Some(interpolation_carve_out(
        &source_text,
        taint,
        ctx.lexer_config(),
    ))
}

/// Select and classify one nested command from its original source component.
fn retained_command_part_taint<S: std::hash::BuildHasher>(
    spelling: &str,
    uses: &HashMap<Symbol, u32>,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    ctx: TaintCtx<'_>,
    depth: u32,
) -> TaintLattice {
    let Some(nested) = retained_taint_substitution(spelling, ctx) else {
        return TaintLattice::tainted();
    };
    let Some(command) = nested.argv_texts.first() else {
        return TaintLattice::tainted();
    };
    let args = nested
        .argv_texts
        .iter()
        .skip(1)
        .cloned()
        .collect::<Vec<_>>();
    substitution_word_taint(spelling, (command, &args), uses, taints, ctx, depth)
}

/// Explicit detached compatibility path; it creates Document text only.
fn standalone_interpolation_taint<S: std::hash::BuildHasher>(
    word: &str,
    stripped: &str,
    uses: &HashMap<Symbol, u32>,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    ctx: TaintCtx<'_>,
    depth: u32,
) -> TaintLattice {
    let Ok(length) = u32::try_from(stripped.len()) else {
        return TaintLattice::tainted();
    };
    let Ok(arena) = tcl_lexer::ExecutablePartArena::decompose(
        tcl_lexer::SourceImage::document(stripped),
        Span::new(0, length),
        tcl_lexer::SubstFlags::default(),
        ctx.lexer_config(),
    ) else {
        return TaintLattice::tainted();
    };
    let mut taint = TaintLattice::clean();
    for part in arena.all_parts() {
        match part.part {
            tcl_lexer::ExecutablePart::Variable { .. } => {
                let Some(name) = arena.bytes(part.span).and_then(|bytes| {
                    tcl_syntax::naming::variable_reference_root_bytes(bytes, ctx.lexer_config())
                        .ok()
                        .flatten()
                        .and_then(|name| std::str::from_utf8(name).ok())
                        .filter(|name| !name.contains('\0'))
                }) else {
                    return TaintLattice::tainted();
                };
                taint = taint.join(var_taint(name, uses, taints, ctx.ssa));
            }
            tcl_lexer::ExecutablePart::Command { .. } => {
                let Some(sub) = arena
                    .bytes(part.span)
                    .and_then(|bytes| std::str::from_utf8(bytes).ok())
                else {
                    return TaintLattice::tainted();
                };
                let relative = usize::try_from(part.span.start())
                    .ok()
                    .and_then(|start| (word.len() - word.trim_start().len()).checked_add(start));
                let sub_ctx = TaintCtx {
                    source_word: None,
                    word_offset: relative
                        .and_then(|relative| u32::try_from(relative).ok())
                        .and_then(|relative| ctx.word_offset?.checked_add(relative)),
                    ..ctx
                };
                taint = taint.join(word_taint_at(sub, uses, taints, sub_ctx, depth + 1));
            }
            tcl_lexer::ExecutablePart::Expression { .. }
            | tcl_lexer::ExecutablePart::ParseError(_) => return TaintLattice::tainted(),
            tcl_lexer::ExecutablePart::Text(_) => {}
        }
    }
    interpolation_carve_out(word, taint, ctx.lexer_config())
}

/// Original word bytes selected by the real module or invocation owner.
/// A compatibility string and an offset alone cannot manufacture a source.
pub(super) fn retained_word_source(
    word: &str,
    ctx: TaintCtx<'_>,
) -> Option<(tcl_lexer::SourceImage, Span)> {
    let image = if let Some(module) = ctx.source.module() {
        module.source.clone()
    } else {
        let tokens = ctx.source_tokens?;
        let binding = tokens.source_binding.as_ref()?;
        binding.original_argument_source_image(tokens)?.clone()
    };
    let span = if let Some(source_word) = ctx.source_word {
        let tokens = ctx.source_tokens?;
        if !tokens.words().contains(source_word)
            || !matches!(
                source_word.source().provenance,
                crate::ir::Provenance::Source
            )
        {
            return None;
        }
        source_word.source().span
    } else {
        let start = ctx.word_offset?;
        let end = start.checked_add(u32::try_from(word.len()).ok()?)?;
        let span = Span::new(start, end);
        (image.bytes().get(span.as_range())? == word.as_bytes()).then_some(())?;
        span
    };
    image.bytes().get(span.as_range())?;
    Some((image, span))
}

/// Resolve a variable component only through its exact source read receipt and
/// SSA read owner. A missing read or represented version remains tainted.
pub(super) fn original_component_read_taint<S: std::hash::BuildHasher>(
    ctx: TaintCtx<'_>,
    span: Span,
    spelling: &str,
    taints: &HashMap<ValueKey, TaintLattice, S>,
) -> TaintLattice {
    let Some(tokens) = ctx.source_tokens else {
        return TaintLattice::tainted();
    };
    // A braced reference's access ends on its last inner character and the
    // component on its closer, so the two agree on the start and spelling.
    let Some(access) = tokens.variable_accesses.iter().find(|access| {
        access.source.span.start() == span.start() && access.original_spelling == spelling
    }) else {
        return TaintLattice::tainted();
    };
    let site = crate::ir::SourceSite::source(access.source.span);
    let mut selected = None;
    for (block, data) in &ctx.ssa.blocks {
        for (index, statement) in data.statements.iter().enumerate() {
            if Some(statement.statement.span().start()) != ctx.source_position {
                continue;
            }
            let point = crate::ssa::SsaSourceView::at_statement(ctx.ssa, *block, index);
            if let Some(read) = point.read_reference(&site, spelling) {
                let current = (read.symbol, read.version);
                if selected.is_some_and(|previous| previous != current) {
                    return TaintLattice::tainted();
                }
                selected = Some(current);
            }
        }
    }
    let Some((symbol, Some(version))) = selected else {
        return TaintLattice::tainted();
    };
    taints
        .get(&(symbol, version))
        .copied()
        .unwrap_or_else(TaintLattice::tainted)
}

/// Re-derive the structural / option-prefix colours of an interpolated
/// (concatenated) word from its literal fragments.
///
/// Interpolation invalidates every structural guarantee unless the
/// literal text re-establishes it: the canonical-list / normalised-path /
/// escaped colours are cleared, `CRLF_FREE` is cleared when a literal
/// fragment contains CR/LF, and the leading literal character controls
/// option-prefix safety (`PATH_PREFIXED` / `NON_DASH_PREFIXED`). A clean
/// join is returned unchanged.
pub(super) fn interpolation_carve_out(
    value: &str,
    joined: TaintLattice,
    config: tcl_lexer::LexerConfig,
) -> TaintLattice {
    if !joined.is_tainted() {
        return TaintLattice::clean();
    }
    let mut colour = joined.colours;
    colour &= !(TaintColour::LIST_CANONICAL
        | TaintColour::PATH_NORMALISED
        | TaintColour::PATH_BOUNDED
        | TaintColour::HEADER_TOKEN_SAFE
        | TaintColour::HTML_ESCAPED
        | TaintColour::URL_ENCODED
        | TaintColour::REGEX_LITERAL
        | TaintColour::SHELL_ATOM);
    if literal_contains_crlf(value, config) {
        colour &= !TaintColour::CRLF_FREE;
    }
    match leading_literal_prefix_char(value, config) {
        Some('/') => colour |= TaintColour::PATH_PREFIXED | TaintColour::NON_DASH_PREFIXED,
        Some('-') => colour &= !(TaintColour::NON_DASH_PREFIXED | TaintColour::PATH_PREFIXED),
        Some(_) => {
            colour |= TaintColour::NON_DASH_PREFIXED;
            colour &= !TaintColour::PATH_PREFIXED;
        }
        None => {}
    }
    TaintLattice {
        colours: colour | TaintColour::TAINTED,
        provenance: if joined.is_conservative_global_seed_only() {
            TaintProvenance::ConservativeGlobalSeedOnly
        } else {
            TaintProvenance::OtherTaint
        },
    }
}

/// Return the leading literal character of `value`, or `None` when the
/// word starts with a variable/command substitution (dynamic prefix).
///
/// The first `Esc` token is rendered through `backslash_subst` (so
/// `\x2f` → `/`); the first `Str` (braced) token contributes its literal
/// first char; a leading `Var` or `Cmd` token means the prefix is
/// dynamic.
pub(super) fn leading_literal_prefix_char(
    value: &str,
    config: tcl_lexer::LexerConfig,
) -> Option<char> {
    let source_map = SourceMap::new(value);
    let tokens = Lexer::with_config(
        value,
        tcl_lexer::SourceSegmentationPurpose::NestedScript.config(config),
    )
    .tokenise_all()
    .ok()?;
    for tok in tokens {
        match tok.kind {
            // End of input, or a dynamic (variable/command) prefix — no
            // literal leading character to report.
            TokenType::Eol | TokenType::Eof | TokenType::Var | TokenType::Cmd => return None,
            TokenType::Esc => {
                let text = source_map.text(tok.span);
                let rendered = if text.contains('\\') {
                    backslash_subst_in(text, config.escapes).into_owned()
                } else {
                    text.to_owned()
                };
                if let Some(c) = rendered.chars().next() {
                    return Some(c);
                }
            }
            TokenType::Str => {
                if let Some(c) = source_map.text(tok.span).chars().next() {
                    return Some(c);
                }
            }
            _ => {}
        }
    }
    None
}

/// Return `true` when any rendered literal fragment of `value` contains a
/// CR or LF.: `Esc` fragments are
/// `backslash_subst`-rendered (so `\n` resolves to a real newline) before
/// the scan.
pub(super) fn literal_contains_crlf(value: &str, config: tcl_lexer::LexerConfig) -> bool {
    let source_map = SourceMap::new(value);
    let Ok(tokens) = Lexer::with_config(
        value,
        tcl_lexer::SourceSegmentationPurpose::NestedScript.config(config),
    )
    .tokenise_all() else {
        return false;
    };
    for tok in tokens {
        match tok.kind {
            TokenType::Eol => return false,
            TokenType::Esc => {
                let text = source_map.text(tok.span);
                let rendered = if text.contains('\\') {
                    backslash_subst_in(text, config.escapes).into_owned()
                } else {
                    text.to_owned()
                };
                if rendered.contains('\r') || rendered.contains('\n') {
                    return true;
                }
            }
            TokenType::Str => {
                let text = source_map.text(tok.span);
                if text.contains('\r') || text.contains('\n') {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}
