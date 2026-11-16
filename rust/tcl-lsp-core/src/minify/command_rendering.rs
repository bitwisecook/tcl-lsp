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

//! One command parsed and re-emitted in its shortest form: words, variable references, quoting and lambda bodies.

#[cfg(test)]
use super::aliasing::standalone_alias_word;
use super::case_lists::minify_case_list;
use super::expressions::compress_expr;
use super::{Arg, MinifyEnv, minify_body};
use tcl_lexer::{LexerConfig, SourceMap, Token, TokenType};
use tcl_registry::ArgRole;

/// Project shared complete groups into the renderer's argument container.
/// Group kinds/ranges and expansion ownership come from the same lexer owner.
pub(super) fn parse_commands(source: &str, tokens: &[Token], config: LexerConfig) -> Vec<Vec<Arg>> {
    // naming.minifier.complete-lexical-head
    // docs/design/analysis/name-resolution-proofs/minifier-complete-lexical-head.md
    tcl_lexer::group_commands(tokens, source, config)
        .into_iter()
        .map(|command| {
            command
                .words
                .into_iter()
                .map(|word| {
                    let mut fragments = command
                        .expand_markers
                        .iter()
                        .copied()
                        .filter(|index| {
                            word.expand
                                && *index < word.tokens.start
                                && tokens[*index].span.end() <= word.span.start()
                                && !tokens[*index + 1..word.tokens.start]
                                    .iter()
                                    .any(|token| token.kind != TokenType::Expand)
                        })
                        .map(|index| tokens[index])
                        .collect::<Vec<_>>();
                    fragments.extend_from_slice(&tokens[word.tokens]);
                    Arg {
                        tokens: fragments,
                        is_braced: word.kind == tcl_lexer::WordKind::Braced,
                        is_quoted: word.kind == tcl_lexer::WordKind::Quoted,
                    }
                })
                .collect()
        })
        .collect()
}

pub(super) fn render_command(
    sm: &SourceMap,
    cmd_args: &[Arg],
    original: Option<&tcl_lexer::NativeScriptCommandWords>,
    env: MinifyEnv<'_>,
    depth: u32,
) -> Option<Vec<String>> {
    let words = original.and_then(|command| {
        tcl_compiler::registry_invocation::source_structure::original_logical_registry_words_for_command(
            env.identities, env.input, command,
        )
    });
    // Effective aliases and expansions retain their original producers. Only
    // a genuine complete written operand can receive a whole-word rewrite.
    let written_role = |index: usize, role: ArgRole| {
        let words = words.as_ref()?;
        let argument = words.roles()?.iter().find_map(|&(argument, selected)| {
            (selected == role
                && words.origins().get(argument + 1)
                    == Some(
                        &tcl_compiler::registry_invocation::InvocationWordOrigin::Written(index),
                    ))
            .then_some(argument)
        })?;
        let operand = words.operands().get(argument)?.as_ref()?.word()?;
        (original?.words.get(index)? == operand).then_some(())
    };
    let bodies = words
        .as_ref()
        .map(|words| words.source_script_bodies(env.context));
    let case = words.as_ref().and_then(|words| {
        words.with_source_schema(env.context, |schema| {
            schema.authored_source_case_invocation()
        })?
    });
    let list_index = case.as_ref().and_then(|(_, layout)| {
        let ordinal = layout.clause_list_index?.checked_add(1)?;
        match words.as_ref()?.origins().get(ordinal)? {
            tcl_compiler::registry_invocation::InvocationWordOrigin::Written(index) => Some(*index),
            _ => None,
        }
    });
    let mut out: Vec<String> = Vec::with_capacity(cmd_args.len());
    for (i, arg) in cmd_args.iter().enumerate() {
        let single_braced = arg.is_braced && arg.tokens.len() == 1;
        let original_word = original.and_then(|command| command.words.get(i));
        let selected_body = original_word.and_then(|word| {
            let content = word.content_span().ok()?;
            bodies
                .as_ref()?
                .iter()
                .find(|body| body.original_container() == word && body.content_span() == content)
        });
        if list_index == Some(i) && single_braced {
            let inner = sm.token_text(arg.tokens[0]);
            let minified =
                minify_case_list(inner, &case.as_ref()?.0, env, depth + 1, bodies.as_deref()?);
            out.push(format!("{{{minified}}}"));
        } else if selected_body.is_some() && single_braced {
            let inner = sm.token_text(arg.tokens[0]);
            out.push(format!(
                "{{{}}}",
                minify_body(
                    inner,
                    env,
                    depth + 1,
                    tcl_lexer::SourceSegmentationPurpose::NestedScript
                )
            ));
        } else if written_role(i, ArgRole::LambdaLiteral).is_some() && single_braced {
            out.push(minify_lambda_literal(
                sm,
                arg.tokens[0],
                original_word?,
                env,
                depth + 1,
            ));
        } else if written_role(i, ArgRole::Expr).is_some() && single_braced {
            let inner = sm.token_text(arg.tokens[0]);
            out.push(format!("{{{}}}", compress_expr(inner, env, depth + 1)));
        } else {
            out.push(reconstruct_arg(sm, arg, original_word, env, depth)?);
        }
    }
    Some(out)
}

/// Original whole-command words under the actual complete body configuration.
#[cfg(test)]
pub(super) fn lexical_script_words(
    source: &str,
    config: LexerConfig,
) -> Option<tcl_lexer::NativeScriptWordsPlan> {
    // naming.minifier.complete-lexical-head
    // docs/design/analysis/name-resolution-proofs/minifier-complete-lexical-head.md
    let image = tcl_lexer::SourceImage::document(source);
    let region = tcl_lexer::Span::new(0, u32::try_from(source.len()).ok()?);
    tcl_lexer::native_script_words_in(image, region, config).ok()
}

/// Static ASCII metadata value of the exact complete original head word.
#[cfg(test)]
pub(super) fn lexical_head_value(
    sm: &SourceMap,
    arg: &Arg,
    word: &tcl_lexer::NativeWord,
    config: LexerConfig,
) -> Option<String> {
    // naming.minifier.complete-lexical-head
    // docs/design/analysis/name-resolution-proofs/minifier-complete-lexical-head.md
    if word.image().channel() != tcl_lexer::SourceChannel::Document
        || word.image().bytes() != sm.source().as_bytes()
        || word.config() != config
        || word.tokens() != arg.tokens.as_slice()
    {
        return None;
    }
    let value = tcl_syntax::word_rules::original_static_word_ascii_presentation(word)?;
    String::from_utf8(value).ok()
}

/// Minify the selected lambda's body, retaining complete configured original
/// field values. A cooked field has no original region or nested role receipt.
pub(super) fn minify_lambda_literal(
    sm: &SourceMap,
    tok: Token,
    word: &tcl_lexer::NativeWord,
    env: MinifyEnv<'_>,
    depth: u32,
) -> String {
    let fallback = || format!("{{{}}}", sm.token_text(tok));
    let Some(elems) =
        tcl_compiler::lambda_literal::split_original_lambda_literal_lexical_decoded(word)
    else {
        return fallback();
    };
    let Some(decoded_body) = elems.body.as_deref() else {
        return fallback();
    };
    let body = tcl_compiler::lambda_literal::split_original_lambda_literal(word)
        .and_then(|lambda| lambda.braced_body())
        .and_then(|span| env.original_source.get(span.as_range()))
        .filter(|body| *body == decoded_body)
        .unwrap_or(decoded_body);
    let minified_body = minify_body(
        body,
        env,
        depth,
        tcl_lexer::SourceSegmentationPurpose::NestedScript,
    );
    let mut fields = vec![elems.params.as_ref(), minified_body.as_str()];
    if let Some(namespace) = elems.namespace.as_deref() {
        fields.push(namespace);
    }
    let value = fields
        .iter()
        .map(|field| tcl_syntax::list::list_element(field))
        .collect::<Vec<_>>()
        .join(" ");
    let rules = tcl_syntax::word_rules::WordValueRules::from_config(&env.config);
    let Ok(reparsed) = rules.split_list(&value) else {
        return fallback();
    };
    if reparsed.len() != fields.len()
        || reparsed
            .iter()
            .zip(&fields)
            .any(|(parsed, field)| parsed.as_ref() != *field)
    {
        return fallback();
    }
    tcl_syntax::word_rules::lexical_unicode_source_word(&value, env.config).unwrap_or_else(fallback)
}

/// Readonly source roles and case shape from the actual retained generation.
#[cfg(test)]
pub(super) fn lexical_source_schema<'r: 'w, 'w, T>(
    env: MinifyEnv<'r>,
    name: &'w str,
    arguments: &'w [tcl_registry::InvocationWord<'w>],
    project: impl FnOnce(&tcl_registry::ResolvedInvocation<'r, 'w>) -> T,
) -> Option<T> {
    // naming.minifier.retained-lexical-context
    // docs/design/analysis/name-resolution-proofs/minifier-retained-lexical-context.md
    if name.is_empty()
        || env.context.commands().snapshot().semantic_key()
            != env.registry.snapshot().semantic_key()
    {
        return None;
    }
    let mut dialect = tcl_registry::InvocationDialect::of_profile(env.dialect);
    dialect.lexer_grammar = env.config.grammar_over(dialect.lexer_grammar);
    let words = tcl_registry::InvocationWords::structured(
        tcl_registry::InvocationWord::Literal(name),
        arguments,
    )
    .with_dialect(dialect);
    tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
        env.registry,
        Some(env.context.context()),
        words,
        tcl_dialect::model::InvocationRealm::RuleLoader,
    )
    .resolved()
    .as_ref()
    .map(project)
}

/// One original reference, preserving arrays and checking scalar proposals against
/// the next rendered component under the complete selected source grammar.
pub(super) fn reconstruct_variable_reference(
    sm: &SourceMap,
    tok: Token,
    following: &str,
    config: LexerConfig,
) -> Option<String> {
    let source = sm.source_bytes();
    let at = tok.span.as_range().start;
    let original = tcl_lexer::word_parts::scan_var_ref(source, at, config).ok()??;
    if original.source_span(source, at, 0)? != tok.span
        || original.name_range_in(source)?.start
            != at.checked_add(usize::from(tok.content_offset))?
    {
        return None;
    }
    let spelling = std::str::from_utf8(source.get(at..original.next)?).ok()?;
    let Some(name) = tcl_syntax::naming::whole_scalar_variable_reference_name(spelling, config)
    else {
        return Some(spelling.to_owned());
    };
    tcl_syntax::naming::scalar_source_reference_followed_by(name, following, config)
}

/// Rebuild source text from a single token, re-adding delimiters
/// and recursively minifying `[…]` substitutions.
pub(super) fn reconstruct_raw(
    sm: &SourceMap,
    tok: Token,
    following: &str,
    env: MinifyEnv<'_>,
    in_quotes: bool,
    depth: u32,
) -> Option<String> {
    Some(match tok.kind {
        // Inside a double-quoted word the caller re-wraps the arg in `"…"`, so a
        // `Str` token (e.g. a lone `$` classified as literal) is string data —
        // emit it verbatim, not brace-wrapped as `{$}`.
        TokenType::Str if in_quotes => sm.text(tok.span).to_owned(),
        TokenType::Str => format!("{{{}}}", sm.token_text(tok)),
        TokenType::Cmd => format!(
            "[{}]",
            minify_body(
                sm.token_text(tok),
                env,
                depth + 1,
                tcl_lexer::SourceSegmentationPurpose::NestedScript
            )
        ),
        TokenType::Var => {
            return reconstruct_variable_reference(sm, tok, following, env.config);
        }
        TokenType::Expand => "{*}".to_owned(),
        TokenType::ExprSugar => format!("$({})", sm.token_text(tok)),
        _ => sm.token_text(tok).to_owned(),
    })
}

#[cfg(test)]
pub(super) fn can_strip_quotes(raw: &str, style: tcl_dialect::BracedVarStyle) -> bool {
    let config = LexerConfig {
        braced_var: style,
        ..LexerConfig::default()
    };
    standalone_alias_word(&format!("\"{raw}\""), config).is_some_and(|word| {
        tcl_syntax::word_rules::lexical_source_word_preserves_components(&word, raw)
    })
}

/// Remove `${…}` references from `raw` so the residual brace check
/// in [`can_strip_quotes`] only sees bare braces.
///
/// Delimited by the document's own `${…}` close rule: under 8.x `${a{b}c}`
/// ends at the first `}`, so `c}` is a *real* residual brace and the quotes
/// must stay; under 9.x the whole thing is one reference and they may go.
/// Scanning by the wrong rule strips quotes that were load-bearing, or
/// keeps ones that were not.
#[cfg(test)]
pub(super) fn strip_braced_var_refs(raw: &str, style: tcl_dialect::BracedVarStyle) -> String {
    let bytes = raw.as_bytes();
    let n = bytes.len();
    let mut out = String::with_capacity(n);
    let mut i = 0;
    while i < n {
        if bytes[i] == b'$'
            && i + 1 < n
            && bytes[i + 1] == b'{'
            && let tcl_lexer::BracedVarEnd::Closed(close) =
                tcl_lexer::braced_var_name_end(bytes, i + 2, style)
        {
            i = close + 1;
            continue;
        }
        let ch_len = utf8_len(bytes[i]);
        out.push_str(&raw[i..i + ch_len]);
        i += ch_len;
    }
    out
}

/// Byte length of the UTF-8 char whose lead byte is `b`.
pub(super) fn utf8_len(b: u8) -> usize {
    if b < 0x80 {
        1
    } else if b >> 5 == 0b110 {
        2
    } else if b >> 4 == 0b1110 {
        3
    } else {
        4
    }
}

/// Rebuild the source text of an argument from its tokens.
pub(super) fn reconstruct_arg(
    sm: &SourceMap,
    arg: &Arg,
    original: Option<&tcl_lexer::NativeWord>,
    env: MinifyEnv<'_>,
    depth: u32,
) -> Option<String> {
    let mut raw = String::new();
    // Every scalar proposal sees all subsequently emitted bytes in its word.
    for &tok in arg.tokens.iter().rev() {
        let component = reconstruct_raw(sm, tok, &raw, env, arg.is_quoted, depth)?;
        raw.insert_str(0, &component);
    }
    if arg.is_quoted
        && !original.is_some_and(|word| {
            tcl_syntax::word_rules::lexical_source_word_preserves_components(word, &raw)
        })
    {
        Some(format!("\"{raw}\""))
    } else {
        Some(raw)
    }
}
