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

//! O100: substituting SCCP-proved constants into `expr` assignments, bare `$var` words and interpolated strings.

use super::super::helpers::expr_simplify::NumericCtx;
use super::super::helpers::spans::{
    original_quoted_word_rewrite_span, original_statement_rewrite_span,
};
use super::super::{Optimisation, PassContext};
use super::command_substitution_folds::{
    is_whole_word_cmd_subst, visit_string_interpolation_cmd_subs,
};
use super::{StatementSource, is_value_safe_bare_word, render_propagation_word};
use crate::compilation_unit::CompilationUnit;
use crate::ir::CommandTokens;
use tcl_core_types::DiagCode;
use tcl_syntax::naming::whole_scalar_variable_reference_name;

/// O100 (`AssignExpr` form): substitute SCCP-proved constants
/// into ``set name [expr { … }]`` expressions.
///
/// Builds on ``substitute_expr_constants`` — the same helper the
/// branch-condition pass uses. Emits an O100 rewrite targeting
/// the whole ``set`` statement with the substituted expression
/// re-wrapped in ``[expr { … }]``. The rewrite span is extended
/// through the original retained command vector so nested substitutions do not
/// leave orphan delimiters. The actual written target and head are retained.
pub(super) fn try_substitute_assign_expr(
    ctx: &mut PassContext<'_>,
    source: StatementSource<'_>,
    span: tcl_lexer::Span,
    expr: &crate::expr_ast::ExprNode,
    constants: &std::collections::HashMap<String, String>,
    numeric: NumericCtx<'_>,
) {
    use super::super::helpers::expr_simplify::{
        expr_has_command_subst, instcombine_expr_typed, substitute_expr_constants_for_execution,
    };
    use crate::tcl_expr_eval::{Env, format_tcl_value_with_policy};

    let Some(rewrite_span) = original_statement_rewrite_span(ctx, span) else {
        return;
    };

    if matches!(expr, crate::expr_ast::ExprNode::Raw { .. }) {
        return;
    }
    if expr_has_command_subst(expr) {
        return;
    }
    // `expr` renamed/aliased anywhere in the module means the source's
    // `[expr {…}]` no longer has builtin semantics — do not propagate a
    // rewrite computed as if it did.
    if !ctx.command_mutations.trusts("expr") {
        return;
    }
    let original = source.tokens.and_then(|tokens| {
        super::super::expr_simplify::OriginalExpressionStatementSource::capture_with_tokens(
            ctx,
            source.statement,
            tokens,
            expr,
        )
    });
    let expr_text = crate::expr_ast::render_expr(expr);
    let result = substitute_expr_constants_for_execution(
        &expr_text,
        constants,
        ctx.dialect,
        &crate::tcl_expr_eval::NativeOperandProofs::new(),
    );
    if !result.changed {
        return;
    }
    // After substitution, try to fold the result to a constant
    // — the cascade where O100 enables O101. When
    // the substituted expression is fully constant we can emit
    // the unwrapped ``set name VALUE`` form directly. Otherwise
    // keep the expression wrapper around the substituted text.
    let env = constants
        .iter()
        .map(|(name, value)| {
            (
                name.clone(),
                crate::tcl_expr_eval::EnvValue::Str(value.clone()),
            )
        })
        .collect::<Env>();
    if let Some(val) = ctx.eval_expression_at(expr, &env, span)
        && let Some(folded) = format_tcl_value_with_policy(&val, ctx.fold_policy())
    {
        let needs_quoting = folded.is_empty()
            || folded.contains([
                ' ', '\t', '\n', '\r', '$', '[', ']', '{', '}', '"', '\\', '\0', ';',
            ]);
        if !needs_quoting
            && let Some(replacement) = original.as_ref().and_then(|source| source.folded(&folded))
        {
            ctx.report(Optimisation::new(
                DiagCode::O100,
                "Propagate constant and fold",
                rewrite_span,
                replacement,
            ));
            return;
        }
    }
    // Not fully constant; try one pass of instcombine on the
    // substituted expression to pick up identity simplifications
    // (e.g. ``$a + 0`` after ``$a`` folds to ``3``).
    let (simplified, _changed) = instcombine_expr_typed(&result.text, false, numeric, ctx.dialect);
    let final_text = if simplified.trim().is_empty() {
        result.text.clone()
    } else {
        simplified
    };
    if ctx
        .expression_rewrite_equivalence_at(expr, &final_text, &env, span)
        .is_err()
    {
        return;
    }
    let Some(replacement) = original
        .as_ref()
        .and_then(|source| source.simplified(&final_text))
    else {
        return;
    };
    ctx.report(Optimisation::new(
        DiagCode::O100,
        "Propagate constant into expr argument",
        rewrite_span,
        replacement,
    ));
}

pub(super) fn visit_call_tokens(
    ctx: &mut PassContext<'_>,
    cu: &CompilationUnit,
    tokens: &CommandTokens,
    constants: &std::collections::HashMap<String, String>,
) {
    for (i, span) in tokens.argv.iter().enumerate() {
        let single = tokens.single_token_word.get(i).copied().unwrap_or(false);
        let Some(text) = tokens.argv_texts.get(i) else {
            continue;
        };
        // A braced `{…}` word (kind `Str`) is a literal / script body: Tcl
        // performs NO substitution inside braces, so neither the simple-`$var`
        // fold nor the `"…"` / `[cmd]` interpolation folds may touch it (else
        // `puts {$x}` is wrongly rewritten to `puts 42`, or a proc body like
        // `{ set d [dict create a 1] }` is spliced into a quoted string).
        if tokens.argv_kinds.get(i).copied() == Some(tcl_lexer::TokenType::Str) {
            continue;
        }
        // A word the lexer reads as one `[…]` is a command substitution, not
        // interpolation text: `visit_call_cmd_subst_folds` folds it through
        // its route, whose decline is final, so neither rewriter below
        // may take it apart — a `]` inside a braced argument
        // (`[string match -nocase {a]€} {€a}]`) is no close of it.
        if single && tokens.argv_kinds.get(i).copied() == Some(tcl_lexer::TokenType::Cmd) {
            continue;
        }
        if single
            && let Some(crate::ir::WordExpr::Variable { spelling, .. }) = tokens.words().get(i)
        {
            visit_simple_var_word(ctx, *span, spelling, constants, cu.ir_module.lexer_config);
        }
        // `"..."` interpolation substitution — works on both
        // single-token (quoted strings) and composite (mixed
        // text + var) words.
        visit_string_interpolation(ctx, *span, text, constants, cu.ir_module.lexer_config);
        // Fold a pure-builtin `[cmd …]` substitution
        // embedded *inside* an interpolation string.
        visit_string_interpolation_cmd_subs(ctx, cu, tokens, *span, text, constants);
    }
}

fn visit_simple_var_word(
    ctx: &mut PassContext<'_>,
    span: tcl_lexer::Span,
    text: &str,
    constants: &std::collections::HashMap<String, String>,
    config: tcl_lexer::LexerConfig,
) {
    let Some(name) = whole_scalar_variable_reference_name(text, config) else {
        return;
    };
    let Some(value) = constants.get(name) else {
        return;
    };
    // Re-render a metacharacter-bearing constant as a
    // single self-contained word instead of bailing.
    let Some(word) = render_propagation_word(value, config) else {
        return;
    };
    ctx.report(Optimisation::new(
        DiagCode::O100,
        "Propagate constant into command argument",
        span,
        word,
    ));
}

/// Inline SCCP-proved constants into a `"…$x…"` string arg.
///
/// Rewrites only when:
/// 1. The argument begins with `"` and ends with `"` (a
///    double-quoted word).
/// 2. Every `$name` / `${name}` reference inside resolves to a
///    constant whose string form is safe for `"…"`
///    interpolation — no `$`, `[`, `\`, or `"`.
///
/// Produces a single `O100` diagnostic spanning the whole arg
/// with the fully-substituted string text.
pub(super) fn visit_string_interpolation(
    ctx: &mut PassContext<'_>,
    span: tcl_lexer::Span,
    text: &str,
    constants: &std::collections::HashMap<String, String>,
    config: tcl_lexer::LexerConfig,
) {
    // The lexer's argv_text strips outer `"…"` delimiters, so
    // we accept either form. Skip strings whose content is a
    // single bare `$var` — that path is already covered by
    // `visit_simple_var_word` with a more appropriate span.
    let inside = text
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(text);
    if !inside.contains('$') {
        return;
    }
    if whole_scalar_variable_reference_name(inside, config).is_some() {
        return;
    }
    // A free-standing `[cmd …]` word is a command substitution, not a
    // `"…"` interpolation: wrapping it in quotes (below) and extending the
    // span to a non-existent close-quote both corrupt the output (e.g.
    // `puts [expr {$a + $b}]` → `puts "[expr {3 + 4}]"]`). Whole-word subs
    // are folded by `visit_call_cmd_subst_folds`; leave them alone here.
    if is_whole_word_cmd_subst(inside) {
        return;
    }
    let Some(rewritten) = substitute_dollar_refs(inside, constants, config.nested()) else {
        return;
    };
    if rewritten == inside {
        return;
    }
    // The argv span for a composite `"$a $b $c"` word holds only
    // the opening-quote token (e.g. `"$` at 5..7). Extend to the
    // matching close quote so the rewrite target covers the full
    // string — otherwise we leave trailing `$b $c"` garbage.
    let (rewrite_span, rewritten) =
        if let Some(rewrite_span) = original_quoted_word_rewrite_span(ctx, span, inside) {
            (rewrite_span, rewritten)
        } else {
            // The argv text spells every reference `${name}`; the word as
            // written is what the edit replaces.
            let Some((rewrite_span, written)) =
                super::super::helpers::spans::original_quoted_word_contents(ctx, span)
            else {
                return;
            };
            match substitute_dollar_refs(&written, constants, config.nested()) {
                Some(rewritten) if rewritten != written => (rewrite_span, rewritten),
                _ => return,
            }
        };
    ctx.report(Optimisation::new(
        DiagCode::O100,
        "Inline constant into string interpolation",
        rewrite_span,
        format!("\"{rewritten}\""),
    ));
}

/// Scan `text` — the inside of a quoted word — for `$name` / `${name}`
/// references and replace each with the corresponding literal from
/// `constants`, rejecting any value that would re-introduce new
/// substitutions. Returns `None` when any reference names something that is
/// not a known constant (a partial substitution would be worse than no
/// substitution), or when the word does not lex.
///
/// The references are the lexer's own `Var` tokens, so only what Tcl
/// substitutes is rewritten: a command substitution is lexed as the script it
/// is, and a braced word in it (`"b:[list {$x}]"`) is a `Str` token whose
/// `$x` stays (#2250), while a `{` inside a quoted word of that script
/// (`"q:[list "foo {$x}"]"`) is literal text and its `$x` is substituted.
/// Unchanged text is copied as `&str` slices, so a multi-byte character keeps
/// its bytes (#2251).
pub(super) fn substitute_dollar_refs(
    text: &str,
    constants: &std::collections::HashMap<String, String>,
    config: tcl_lexer::LexerConfig,
) -> Option<String> {
    let word = format!("\"{text}\"");
    let mut refs = Vec::new();
    collect_var_refs(&word, config, 0, 0, &mut refs)?;
    refs.sort_unstable_by_key(|r| r.start);
    let mut out = String::with_capacity(text.len());
    let mut copied = 0;
    for r in refs {
        // Offsets are into `word`, one past `text`'s.
        let (start, end) = (r.start - 1, r.end - 1);
        let value = constants.get(&r.name)?;
        if value.contains(['$', '[', '\\', '"']) {
            return None;
        }
        // Inside a `[…]` command substitution the value becomes a command
        // word, so it must be a single self-contained word — a value with
        // whitespace (a list literal like `tran 1n 100n uic`) would split
        // into multiple arguments. Plain string-text occurrences
        // (`cmd_depth == 0`) inline the value verbatim.
        if r.cmd_depth > 0 && !is_value_safe_bare_word(value) {
            return None;
        }
        out.push_str(text.get(copied..start)?);
        out.push_str(value);
        copied = end;
    }
    out.push_str(text.get(copied..)?);
    Some(out)
}

/// One variable substitution [`substitute_dollar_refs`] may rewrite: its byte
/// range in the lexed word, the variable it names, and how many command
/// substitutions enclose it.
struct VarRef {
    pub(super) start: usize,
    pub(super) end: usize,
    pub(super) name: String,
    cmd_depth: u32,
}

/// Collect the `Var` tokens of `src` (a script or a quoted word, lexed under
/// `config`) at offset `base` into `out`, recursing into each command
/// substitution's script. A braced word is a `Str` token and holds none.
/// `None` when the text does not lex or holds a substitution this rewrite
/// cannot map (Jim's `$(…)`).
fn collect_var_refs(
    src: &str,
    config: tcl_lexer::LexerConfig,
    base: usize,
    cmd_depth: u32,
    out: &mut Vec<VarRef>,
) -> Option<()> {
    use tcl_lexer::TokenType;
    if crate::depth_guard::MAX_BRACKET_TEXT_DEPTH.exceeded(cmd_depth) {
        return None;
    }
    let sm = tcl_lexer::SourceMap::new(src);
    let tokens = tcl_lexer::Lexer::with_config(src, config)
        .tokenise_all()
        .ok()?;
    for tok in tokens {
        let (start, end) = (tok.span.start() as usize, tok.span.end() as usize);
        match tok.kind {
            TokenType::Var => {
                let name = sm.token_text(tok);
                // An element index that substitutes (`$arr($idx)`, `$a([k])`,
                // `$a(\x)`) names the element its value selects, not the one
                // its spelling does, so it is left as written.
                if tok.content_offset != 2
                    && tcl_syntax::naming::split_element_ref(name)
                        .is_some_and(|(_, index)| index.contains(['$', '[', '\\']))
                {
                    continue;
                }
                // A `${name}` span stops before its closing brace, but the
                // empty `${}` span already covers it.
                let close =
                    usize::from(tok.content_offset == 2 && src.get(start..end) != Some("${}"));
                out.push(VarRef {
                    start: base + start,
                    end: base + end + close,
                    name: name.to_owned(),
                    cmd_depth,
                });
            }
            TokenType::Cmd => {
                let inner = start + usize::from(tok.content_offset);
                collect_var_refs(
                    sm.token_text(tok),
                    config.nested(),
                    base + inner,
                    cmd_depth + 1,
                    out,
                )?;
            }
            TokenType::ExprSugar => return None,
            _ => {}
        }
    }
    Some(())
}
