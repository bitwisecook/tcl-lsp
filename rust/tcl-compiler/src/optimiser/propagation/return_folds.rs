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

//! Folding the operand of an original `return` and splicing the replacement word into its retained source.

use super::super::helpers::spans::original_statement_rewrite_span;
use super::super::{Optimisation, PassContext};
use super::command_substitution_folds::{retained_o115, retained_substitution_calls};
use super::render_propagation_word;
use crate::ir::{CommandTokens, Statement};
use tcl_core_types::DiagCode;
use tcl_syntax::naming::whole_scalar_variable_reference_name;

/// Replace only the retained original result word of a simple lowered return.
/// This constructs source; the caller must independently approve the fold.
pub(super) fn original_return_value_replacement(
    ctx: &PassContext<'_>,
    statement: &Statement,
    tokens: &CommandTokens,
    value: &str,
) -> Option<(tcl_lexer::Span, String)> {
    let replacement = render_propagation_word(value, ctx.lexer_config())?;
    original_return_word_replacement(ctx, statement, tokens, &replacement)
}

/// Splice one deliberately constructed complete word into the same original
/// return's retained result ordinal. This supplies source geometry only.
pub(super) fn original_return_word_replacement(
    ctx: &PassContext<'_>,
    statement: &Statement,
    tokens: &CommandTokens,
    replacement: &str,
) -> Option<(tcl_lexer::Span, String)> {
    let image = tcl_lexer::SourceImage::document(replacement);
    let extent = tcl_lexer::Span::new(0, u32::try_from(replacement.len()).ok()?);
    let plan = tcl_lexer::native_script_words_in(image, extent, ctx.lexer_config()).ok()?;
    if plan.fatal_tail.is_some() {
        return None;
    }
    let [proposed] = plan.commands.as_slice() else {
        return None;
    };
    let [proposed_word] = proposed.words.as_slice() else {
        return None;
    };
    if proposed_word.span() != extent || proposed_word.group().expand {
        return None;
    }
    let Statement::Return {
        value_word: Some(value_word),
        ..
    } = statement
    else {
        return None;
    };
    let module = ctx.ir_module?;
    let registry = ctx.registry?;
    let config = ctx.lexer_config();
    let span = original_statement_rewrite_span(ctx, statement.span())?;
    let binding = tokens.source_binding.as_ref()?;
    binding.original_invocation_metadata_for_module(tokens, module, registry)?;
    let site = binding.invocation_site()?;
    if site.offset != span.start() || site.source.source_image() != &module.source {
        return None;
    }
    let command = site
        .source
        .original_lexical_command(tokens.words(), site.offset, config)?;
    let mut matches = tokens
        .words()
        .iter()
        .enumerate()
        .filter(|(_, word)| *word == value_word);
    let (written, _) = matches.next()?;
    if matches.next().is_some() {
        return None;
    }
    let word = command.native_words().get(written)?;
    let start = usize::try_from(word.span().start().checked_sub(span.start())?).ok()?;
    let end = usize::try_from(word.span().end().checked_sub(span.start())?).ok()?;
    let mut source = ctx
        .source
        .get(span.start() as usize..span.end() as usize)?
        .to_owned();
    source.get(start..end)?;
    source.replace_range(start..end, replacement);
    // Full proposed statement parsing detects any quote/continuation seam.
    let image = tcl_lexer::SourceImage::document(&source);
    let extent = tcl_lexer::Span::new(0, u32::try_from(source.len()).ok()?);
    let after = tcl_lexer::native_script_words_in(image, extent, config).ok()?;
    if after.fatal_tail.is_some() {
        return None;
    }
    let [after] = after.commands.as_slice() else {
        return None;
    };
    (after.words.len() == command.native_words().len()).then_some((span, source))
}

/// Fold an original return operand using its retained word kind and constants.
pub(super) fn try_fold_return_terminator(
    ctx: &mut PassContext<'_>,
    statement: &Statement,
    constants: &std::collections::HashMap<String, String>,
    module: &crate::ir::Module,
    tokens: Option<&CommandTokens>,
) {
    let Statement::Return {
        span,
        value,
        value_word,
        expr,
        ..
    } = statement
    else {
        return;
    };
    let span = *span;
    let value = value.as_deref();
    let expr = expr.as_ref();

    // O115: `return [expr {[expr {E}]}]` → `return [expr {E}]`. Checked
    // before the `expr.is_some()` early-return below because the return
    // value of a cmd-sub also populates `expr`, yet the redundant-nested-
    // expr collapse operates on the raw value text. Also requires `expr`
    // to be untouched anywhere in the module (mirrors the O129
    // builtin-fold trust check) — both the outer and inner `[expr {…}]`
    // are genuine command substitutions, and a shadowed `expr` no longer
    // has builtin semantics.
    let collapse = tokens.and_then(|tokens| {
        let registry = ctx.registry?;
        let calls = retained_substitution_calls(tokens, module, registry)?;
        let raw = value?.trim();
        let mut matching = calls.iter().filter(|call| {
            ctx.source
                .get(call.span.start() as usize..call.span.end() as usize)
                == Some(raw)
        });
        let call = matching.next()?;
        if matching.next().is_some() {
            return None;
        }
        retained_o115(ctx, module, tokens, call, raw, registry)
    });
    if let Some(collapsed) = collapse
        && let Some((span, replacement)) = tokens
            .and_then(|tokens| original_return_word_replacement(ctx, statement, tokens, &collapsed))
    {
        ctx.report(Optimisation::new(
            DiagCode::O115,
            "Remove redundant nested expr",
            span,
            replacement,
        ));
        return;
    }

    if let Some(expression) = expr {
        let env = constants
            .iter()
            .map(|(name, value)| {
                (
                    name.clone(),
                    crate::tcl_expr_eval::EnvValue::Str(value.clone()),
                )
            })
            .collect::<crate::tcl_expr_eval::Env>();
        if let Some(value) = ctx.eval_expression_at(expression, &env, span)
            && let Some(folded) =
                crate::tcl_expr_eval::format_tcl_value_with_policy(&value, ctx.fold_policy())
            && !folded.contains(['$', '['])
            && let Some((rewrite_span, replacement)) = tokens.and_then(|tokens| {
                original_return_value_replacement(ctx, statement, tokens, &folded)
            })
        {
            ctx.report(Optimisation::new(
                DiagCode::O101,
                "Fold constant expression",
                rewrite_span,
                replacement,
            ));
            return;
        }
    }

    // Only fold `return $v` — numeric/bare literals and complex
    // values are left to richer passes.
    if expr.is_some() {
        return;
    }
    let Some(crate::ir::WordExpr::Variable { spelling, .. }) = value_word.as_ref() else {
        return;
    };
    let Some(name) = whole_scalar_variable_reference_name(spelling, module.lexer_config) else {
        return;
    };
    let Some(resolved) = constants.get(name) else {
        return;
    };
    // Render the constant as a single self-contained word
    // rather than bailing on metacharacters (`return {Hello World}`).
    let Some((rewrite_span, replacement)) = tokens
        .and_then(|tokens| original_return_value_replacement(ctx, statement, tokens, resolved))
    else {
        return;
    };
    ctx.report(Optimisation::new(
        DiagCode::O100,
        "Fold return of constant variable",
        rewrite_span,
        replacement,
    ));
}
