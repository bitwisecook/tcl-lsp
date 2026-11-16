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

//! Tokens for one argument word and for expression operands.

use super::argument_override::{ArgOverride, subspec_content};
use super::command_tokens::classify_and_push_if;
use super::emission::{Entry, push_subtoken, push_token};
use super::format_subtokens::{
    push_binary_subtokens, push_clock_subtokens, push_escape_subtokens, push_sprintf_subtokens,
};
use super::legend::{MOD_DECLARATION, MOD_DEFAULT_LIBRARY, MOD_DEFINITION, TokenKind};
use super::regex_subtokens::{push_regex_subtokens, push_regsub_subtokens};
use super::script_walk::{
    ScriptCtx, collect_case_list, collect_lambda_literal, collect_loop_var_list,
    collect_param_list, collect_script,
};
use crate::definition::utf16_len;
use tcl_dialect::NumberSyntax;
use tcl_lexer::{Token, TokenType};

/// The fixed `(kind, modifier)` for overrides that emit their token verbatim,
/// or `None` for overrides that need custom handling (recursion / sub-tokens).
fn verbatim_token_kind(ov: &ArgOverride) -> Option<(TokenKind, u32)> {
    match ov {
        ArgOverride::Kind(kind) => Some((*kind, 0)),
        ArgOverride::Decorator => Some((TokenKind::Decorator, 0)),
        ArgOverride::VarDecl => Some((TokenKind::Variable, MOD_DECLARATION)),
        ArgOverride::VarRef => Some((TokenKind::Variable, 0)),
        ArgOverride::CommandRef => Some((TokenKind::Function, 0)),
        ArgOverride::SubcommandKeyword => Some((TokenKind::Keyword, MOD_DEFAULT_LIBRARY)),
        ArgOverride::ProcNameDef => Some((TokenKind::Function, MOD_DEFINITION)),
        ArgOverride::ClassNameDef => Some((TokenKind::Class, MOD_DEFINITION)),
        ArgOverride::ClassNameRef => Some((TokenKind::Class, 0)),
        _ => None,
    }
}

pub(super) fn emit_arg_token(
    ctx: ScriptCtx<'_>,
    body_ctx: ScriptCtx<'_>,
    tok: Token,
    override_kind: Option<&ArgOverride>,
    deferred_role: bool,
    entries: &mut Vec<Entry>,
    depth: u32,
) {
    let full_source = ctx.full_source;
    let line_index = ctx.line_index;
    let tok = &tok;
    // Substitutions retain the genuine source vocabulary parent; each region
    // issuer checks its own complete argv.
    let plain_ctx = ScriptCtx {
        oo_grammar: None,
        original_definition_members: None,
        declared_words: None,
        ..ctx
    };
    // Overrides that emit their token verbatim collapse to one path.
    if let Some((kind, modifier)) = override_kind.and_then(verbatim_token_kind) {
        push_token(line_index, full_source, *tok, kind, modifier, entries);
        return;
    }
    match override_kind {
        Some(ArgOverride::RegexPattern) => {
            if !push_regex_subtokens(line_index, full_source, *tok, entries) {
                push_token(line_index, full_source, *tok, TokenKind::Regexp, 0, entries);
            }
        }
        Some(ArgOverride::SprintfFormat) => {
            let emitted =
                push_sprintf_subtokens(line_index, full_source, *tok, ctx.dialect, entries);
            classify_and_push_if(!emitted, ctx, *tok, entries);
        }
        Some(ArgOverride::ClockFormat) => {
            let emitted = push_clock_subtokens(line_index, full_source, *tok, entries);
            classify_and_push_if(!emitted, ctx, *tok, entries);
        }
        Some(ArgOverride::BinaryFormat) => {
            let emitted =
                push_binary_subtokens(line_index, full_source, *tok, ctx.dialect, entries);
            classify_and_push_if(!emitted, ctx, *tok, entries);
        }
        Some(ArgOverride::RegsubReplace) => {
            let emitted = push_regsub_subtokens(line_index, full_source, *tok, entries);
            classify_and_push_if(!emitted, ctx, *tok, entries);
        }
        Some(ArgOverride::LambdaLiteral) => {
            collect_lambda_literal(ctx, *tok, entries, depth + 1);
        }
        Some(ArgOverride::LoopVarList) => {
            collect_loop_var_list(ctx, *tok, entries);
        }
        Some(ArgOverride::ParamList) => {
            collect_param_list(ctx, *tok, entries);
        }
        Some(ArgOverride::MemberName) => {
            push_token(
                line_index,
                full_source,
                *tok,
                TokenKind::Method,
                MOD_DEFINITION,
                entries,
            );
        }
        Some(ArgOverride::BodyScript) => {
            if let Some((cstart, inner)) = subspec_content(full_source, *tok) {
                // Recurse with the OO-body context computed for this
                // command's bodies (`body_ctx`) so a method / constructor /
                // property-accessor body inside a class definition is walked
                // as ordinary code, while the class body itself stays in OO
                // context.
                collect_script(
                    body_ctx,
                    inner,
                    u32::try_from(cstart).unwrap_or(0),
                    entries,
                    depth + 1,
                    false,
                );
            } else if let Some(kind) = classify_arg_token(*tok, full_source, ctx.numbers) {
                push_token(line_index, full_source, *tok, kind, 0, entries);
            }
        }
        Some(ArgOverride::ExprScript) => {
            collect_expr(plain_ctx, *tok, entries, depth + 1);
        }
        Some(ArgOverride::CaseList(spec, regexp)) => {
            // The written-word check reads this argument's own operand; the
            // clause bodies keep the command's body context.
            let list_ctx = ScriptCtx {
                operand: ctx.operand,
                ..body_ctx
            };
            // A command with no retained invocation reads its written list.
            let written = ctx.original_words.is_none() && ctx.declared_words.is_none();
            collect_case_list(list_ctx, *tok, entries, depth + 1, spec, *regexp, written);
        }
        Some(ArgOverride::KeywordArg) => {
            push_keyword_arg(ctx, *tok, entries);
        }
        // Verbatim-token overrides are handled by the early return above.
        Some(
            ArgOverride::Kind(_)
            | ArgOverride::Decorator
            | ArgOverride::VarDecl
            | ArgOverride::VarRef
            | ArgOverride::CommandRef
            | ArgOverride::SubcommandKeyword
            | ArgOverride::ProcNameDef
            | ArgOverride::ClassNameDef
            | ArgOverride::ClassNameRef,
        ) => {}
        None => emit_default_arg_token(plain_ctx, *tok, entries, depth, deferred_role),
    }
}

/// Handle an argument token with no [`ArgOverride`]: recurse into a `[…]`
/// command substitution, or classify a plain word (splitting backslash
/// escapes out of string literals).  Extracted from [`emit_arg_token`].
///
/// `deferred_role` is `tok`'s own [`collect_script`]-computed deferred-role
/// flag (see there) — threaded through unchanged so the `[…]` recursion below
/// carries it into the substitution's content.
fn emit_default_arg_token(
    ctx: ScriptCtx<'_>,
    tok: Token,
    entries: &mut Vec<Entry>,
    depth: u32,
    deferred_role: bool,
) {
    let full_source = ctx.full_source;
    let line_index = ctx.line_index;
    if matches!(tok.kind, TokenType::Cmd) {
        // Command substitution `[…]` — recurse into the inner
        // script (delimiters stripped via `content_offset`).
        let cstart = tok.span.start() as usize + tok.content_offset as usize;
        let cend = (tok.span.end() as usize).min(full_source.len());
        if cend > cstart
            && let Some(inner) = full_source.get(cstart..cend)
        {
            collect_script(
                ctx,
                inner,
                u32::try_from(cstart).unwrap_or(0),
                entries,
                depth + 1,
                deferred_role,
            );
        }
    } else if let Some(kind) = classify_arg_token(tok, full_source, ctx.numbers) {
        // String / bareword args with backslash escapes split
        // into literal `String` runs + `Escape` sub-tokens.
        if kind == TokenKind::String
            && push_escape_subtokens(line_index, full_source, tok, ctx.config, entries)
        {
            // emitted as sub-tokens
        } else {
            push_token(line_index, full_source, tok, kind, 0, entries);
        }
    }
}

/// Tokenise a braced expression argument via the expression sub-lexer,
/// emitting variable / number / operator / function / string / boolean
/// sub-tokens (math functions carry `defaultLibrary`) and recursing into
/// nested `[cmd]` substitutions.
fn collect_expr(ctx: ScriptCtx<'_>, tok: Token, entries: &mut Vec<Entry>, depth: u32) {
    let full_source = ctx.full_source;
    let line_index = ctx.line_index;
    let Some((cstart, inner)) = subspec_content(full_source, tok) else {
        if let Some(kind) = classify_arg_token(tok, full_source, ctx.numbers) {
            push_token(line_index, full_source, tok, kind, 0, entries);
        }
        return;
    };
    let (tokens, _) = tcl_lexer::tokenise_expr_checked_with_expression_grammar(
        inner,
        &ctx.config.grammar_over(ctx.dialect.grammar),
        ctx.dialect.expr_grammar_base,
        ctx.dialect.f5_core_expr_grammar(),
    );
    for et in tokens {
        use tcl_lexer::ExprTokenType as E;
        let abs_start = cstart + et.start as usize;
        match et.kind {
            E::Command => {
                let original = tcl_lexer::ExprToken {
                    kind: et.kind,
                    text: et.text.as_bytes().to_vec(),
                    start: et.start,
                    end: et.end,
                };
                let Some(term) =
                    tcl_lexer::expression_term(inner.as_bytes(), &original, ctx.config)
                else {
                    continue;
                };
                let Some(body) = inner.get(term.value.start as usize..term.value.end as usize)
                else {
                    continue;
                };
                let Some(offset) = u32::try_from(cstart)
                    .ok()
                    .and_then(|start| start.checked_add(term.value.start))
                else {
                    continue;
                };
                collect_script(ctx, body, offset, entries, depth + 1, false);
            }
            E::Function if !et.text.is_empty() && !et.text.contains('\n') => {
                let pos = line_index
                    .position_at_utf16(u32::try_from(abs_start).unwrap_or(0), full_source);
                let mods = if ctx.lexical {
                    u32::from(
                        ctx.context.source_math_function(&et.text)
                            == tcl_dialect::model::CapabilityAnswer::Yes,
                    ) * MOD_DEFAULT_LIBRARY
                } else {
                    ctx.analysis
                        .and_then(|analysis| {
                            crate::math_function_symbol::at(
                                full_source,
                                analysis,
                                u32::try_from(abs_start).ok()?,
                            )
                        })
                        .and_then(|function| {
                            function
                                .selected_registry_spec(ctx.registry)
                                .map(|_| MOD_DEFAULT_LIBRARY)
                        })
                        .unwrap_or(0)
                };
                entries.push((
                    pos.line,
                    pos.character.get(),
                    utf16_len(&et.text),
                    TokenKind::Function,
                    mods,
                ));
            }
            kind => {
                if let Some(token_kind) = expr_subtoken_kind(kind) {
                    push_subtoken(
                        full_source,
                        line_index,
                        abs_start,
                        &et.text,
                        token_kind,
                        entries,
                    );
                }
            }
        }
    }
}

/// The semantic-token kind an expression sub-token highlights as, for the kinds
/// that map to one directly.
///
/// `None` for the two [`collect_expr`] handles itself — `Command` recurses into
/// the nested script and `Function` needs the `defaultLibrary` modifier — and
/// for the kinds that carry no highlighting (whitespace, the never-emitted
/// `Eof`). Exhaustive on purpose: a new `ExprTokenType` variant fails to compile
/// here rather than silently going unstyled.
fn expr_subtoken_kind(kind: tcl_lexer::ExprTokenType) -> Option<TokenKind> {
    use tcl_lexer::ExprTokenType as E;
    Some(match kind {
        E::Number => TokenKind::Number,
        E::Variable => TokenKind::Variable,
        E::String => TokenKind::String,
        E::Bool => TokenKind::Keyword,
        // The grouping / ternary / argument-separator punctuation is as much an
        // operator as `+` or `&&`, and the expr lexer already tells them apart —
        // dropping them into a catch-all arm would leave
        // `expr {($a + $b) * $c}`'s parens unstyled and
        // `$a > 1 ? "y" : "n"`'s `?` and `:` unstyled.
        E::Operator | E::ParenOpen | E::ParenClose | E::Comma | E::TernaryQ | E::TernaryC => {
            TokenKind::Operator
        }
        // A TIP 582 `# …` comment inside the expression body highlights as a
        // comment, like a script-level one. Always single-line: the lexer stops
        // the token before the terminating newline, so this satisfies
        // `push_subtoken`'s one-line requirement by construction.
        E::Comment => TokenKind::Comment,
        E::Command | E::Function | E::Whitespace | E::Eof => return None,
    })
}

/// Classify a non-head token by its lexer-assigned kind.  `numbers` is the
/// document dialect's numeric-literal grammar, which decides whether a bareword
/// reads as a number (`0o17` is a number from 8.5, a bareword before it).
pub(super) fn classify_arg_token(
    tok: Token,
    source: &str,
    numbers: NumberSyntax,
) -> Option<TokenKind> {
    let span = tok.span;
    let len = (span.end() - span.start()) as usize;
    if len == 0 {
        return None;
    }
    match tok.kind {
        TokenType::Var => Some(TokenKind::Variable),
        TokenType::Str => Some(TokenKind::String),
        TokenType::Esc => {
            // Quoted strings vs barewords vs numbers.  The
            // lexer sets `tok.in_quote = true` on every Esc /
            // Var / Cmd token emitted from inside `"..."`, so
            // multi-fragment quoted strings (e.g. `"a $b c"`)
            // get every literal fragment classified as String
            // — including the leading fragment whose span may
            // not include the opening `"`.  This matches the
            // lexer contract and avoids the prior byte-peek
            // heuristic that missed inner fragments.
            if tok.in_quote {
                return Some(TokenKind::String);
            }
            let start = span.start() as usize;
            let text = source
                .get(start..(start + len).min(source.len()))
                .unwrap_or("");
            if tcl_syntax::number::is_whole_number(text, numbers) {
                Some(TokenKind::Number)
            } else if tcl_syntax::naming::is_qualified(text.as_bytes()) && tok.content_offset == 0 {
                // Only a *bare* word can be a namespace reference.  A quoted or
                // braced word is a string literal even when its content happens
                // to contain `::` — painting `append cmd "::scan \$field"`'s
                // whole quoted word as a namespace would also lose the `\$`
                // escape inside it.
                Some(TokenKind::Namespace)
            } else {
                // Bareword argument words classify as String, so `puts
                // hello` emits the `hello` string token rather than
                // dropping it.
                Some(TokenKind::String)
            }
        }
        _ => None,
    }
}

/// A current original operand owns its content delimiters independently of
/// keyword selection. Missing retained operands keep the token-only syntax view.
pub(super) fn keyword_operand_content<'a>(
    source: &'a str,
    tok: Token,
    original: Option<&tcl_lexer::NativeWord>,
    config: tcl_lexer::LexerConfig,
) -> Option<(usize, &'a str)> {
    if let Some(word) = original {
        if word.image().bytes() != source.as_bytes()
            || word.config().nested() != config.nested()
            || word.group().expand
            || !word.tokens().contains(&tok)
        {
            return None;
        }
        let span = word.content_span().ok()?;
        return source
            .get(span.as_range())
            .map(|content| (span.start() as usize, content));
    }
    subspec_content(source, tok)
}

/// Emit the already selected structural keyword using its original content
/// extent. Literal closing bytes inside that extent remain data.
fn push_keyword_arg(ctx: ScriptCtx<'_>, tok: Token, entries: &mut Vec<Entry>) {
    let original = ctx.operand.and_then(|operand| operand.word.as_ref());
    if let Some((cstart, content)) =
        keyword_operand_content(ctx.full_source, tok, original, ctx.config)
    {
        if !content.is_empty() {
            push_subtoken(
                ctx.full_source,
                ctx.line_index,
                cstart,
                content,
                TokenKind::Keyword,
                entries,
            );
            return;
        }
    } else if original.is_some() {
        return;
    }
    push_token(
        ctx.line_index,
        ctx.full_source,
        tok,
        TokenKind::Keyword,
        0,
        entries,
    );
}
