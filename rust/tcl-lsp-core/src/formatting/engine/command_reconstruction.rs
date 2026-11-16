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

//! One command rebuilt as formatted text: inline bodies, word arguments, keyword rewrites and case lists.

use super::super::config::FormatterConfig;
use super::argument_roles::identify_expr_args;
use super::line_wrapping::{current_col, estimate_trailing_len, split_long_line, wrap_braced_expr};
use super::word_source::{
    native_param_list_source, normalise_backslash_newline, normalise_param_list,
    original_lambda_parameters_match, reconstruct_arg, render_param_list_arg,
};
use super::{ArgKind, CommandArg, ParsedCommand};
use tcl_compiler::lambda_literal::{
    DecodedLambdaLiteral, split_original_lambda_literal_decoded,
    split_original_lambda_literal_lexical_decoded,
};
use tcl_lexer::{SourceMap, TokenType};
use tcl_registry::{CaseListSpec, CommandRegistry, Traits};

// Inline body detection

/// Count complete commands through the shared lexer under the full body grammar.
/// Unavailable source remains outside the inline presentation threshold.
pub(super) fn count_body_commands(
    body_text: &str,
    config: tcl_lexer::LexerConfig,
) -> Option<usize> {
    // naming.editor.original-source-whitespace-geometry
    // docs/design/analysis/name-resolution-proofs/original-source-whitespace-geometry.md
    let image = tcl_lexer::SourceImage::document(body_text);
    let end = u32::try_from(body_text.len()).ok()?;
    super::super::source_layout::words_in(&image, tcl_lexer::Span::new(0, end), config)
        .map(|plan| plan.commands.len())
}

/// Whether a body is short enough to keep on one line.
fn body_can_be_inline(
    body_text: &str,
    config: &FormatterConfig,
    current_line_len: usize,
    never_inline: bool,
) -> bool {
    if config.expand_single_line_bodies
        && count_body_commands(body_text, config.lexer_config())
            .is_none_or(|count| count >= config.min_body_commands_for_expansion)
    {
        // Only force expansion once the body carries at least
        // `min_body_commands_for_expansion` commands; a trivial one-command
        // body stays inline.
        return false;
    }
    let stripped = body_text.trim();
    if stripped.is_empty() {
        return true;
    }
    if never_inline {
        return false;
    }
    if stripped.contains('\n') || stripped.contains('{') || stripped.contains('}') {
        return false;
    }
    let inline_len = current_line_len + "{ ".len() + stripped.len() + " }".len();
    inline_len <= config.goal_line_length
}

// Command reconstruction

/// Push a leading separator space before the next part, unless
/// we're continuing a `}{` brace chain with
/// `space_between_braces == false`.
fn maybe_space(parts: &mut Vec<String>, in_brace_chain: bool, space_between_braces: bool) {
    let suppress = in_brace_chain && !space_between_braces;
    if !suppress && !parts.is_empty() {
        parts.push(" ".to_owned());
    }
}

/// Render a formatted body argument (inline `{ … }` or expanded
/// K&R block) into `parts`.  The caller has already emitted any
/// required leading separator space.
fn append_body_no_space(
    parts: &mut Vec<String>,
    body: &str,
    config: &FormatterConfig,
    indent: &str,
    current_line_len: usize,
    never_inline: bool,
) {
    let inline = body_can_be_inline(body, config, current_line_len, never_inline);
    let stripped = body.trim();
    if stripped.is_empty() {
        parts.push("{}".to_owned());
    } else if inline {
        parts.push(format!("{{ {stripped} }}"));
    } else {
        parts.push("{\n".to_owned());
        parts.push(body.to_owned());
        parts.push(format!("\n{indent}}}"));
    }
}

/// Reassemble an `ArgKind::LambdaLiteral` argument: its parameter-list
/// element (normalised, not code) and its body element (`arg.formatted_body`
/// — already recursively formatted by the caller from just the *decoded*
/// body span, never the whole lambda literal, so the parameter word is
/// never misread as a command name), plus an optional namespace element,
/// wrapped back in the outer `{}`. Falls back to the original literal
/// verbatim if the source can no longer be split (should not happen once
/// `identify_body_args` and this function agree, but a stale token must not
/// panic).
///
/// The parameter-list and namespace elements are decoded (backslash escapes
/// collapsed for a bare/quoted element — [`tcl_compiler::lambda_literal::split_lambda_literal_decoded`])
/// before use, not pasted through as raw source spelling: a non-literal
/// element's escapes would otherwise survive reformatting and change what it
/// means. The namespace is re-quoted with
/// [`tcl_syntax::list::list_element`] on reassembly rather than always left
/// bare, so an element that needs quoting (e.g. an escaped space) still
/// round-trips safely. Native parameters use their retained nested list
/// children, then the complete spelling proposal is checked against the same
/// original native values. Missing or unrepresentable children preserve the
/// whole original word; explicit Logical compatibility uses the String split.
pub(super) fn decoded_lambda(
    arg: &CommandArg,
    logical: bool,
) -> Option<DecodedLambdaLiteral<'static>> {
    match (&arg.original_word, &arg.original_input) {
        (Some(word), Some(input)) => split_original_lambda_literal_decoded(word, input),
        (Some(word), None) if logical => split_original_lambda_literal_lexical_decoded(word),
        _ => None,
    }
}

pub(super) fn render_lambda_literal_arg(
    sm: &SourceMap,
    arg: &CommandArg,
    config: &FormatterConfig,
    indent: &str,
    current_line_len: usize,
    never_inline: bool,
    logical: bool,
) -> String {
    // Implementation contract: naming.editor.original-native-formal-list-formatting
    // docs/design/analysis/name-resolution-proofs/original-native-formal-list-formatting.md
    let fallback = || reconstruct_arg(sm, arg, false);
    let Some(elems) = decoded_lambda(arg, logical) else {
        return fallback();
    };
    let Some(formatted_body) = arg.formatted_body.as_deref() else {
        return fallback();
    };
    let mut body_parts: Vec<String> = Vec::new();
    append_body_no_space(
        &mut body_parts,
        formatted_body,
        config,
        indent,
        current_line_len,
        never_inline,
    );
    let params_rendered = match (&arg.original_word, &arg.original_input) {
        (Some(_), Some(input)) => {
            let Some(params) = input
                .original_list_element(0)
                .and_then(|params| native_param_list_source(&params))
            else {
                return fallback();
            };
            params
        }
        (Some(word), None) if logical && word.config() == config.lexer_config() => {
            normalise_param_list(&elems.params, config.word_rules())
        }
        _ => return fallback(),
    };
    let body_rendered = body_parts.concat();
    let rendered = match elems.namespace.as_deref() {
        Some(ns) => {
            let ns_rendered = tcl_syntax::list::list_element(ns);
            format!("{{{params_rendered} {body_rendered} {ns_rendered}}}")
        }
        None => format!("{{{params_rendered} {body_rendered}}}"),
    };
    if let (Some(word), Some(input)) = (&arg.original_word, &arg.original_input)
        && original_lambda_parameters_match(&rendered, word, input, config).is_none()
    {
        return fallback();
    }
    rendered
}

/// Everything `append_word_arg` needs about the argument it is emitting,
/// beyond the mutable output buffer.
struct WordArgContext<'a> {
    sm: &'a SourceMap<'a>,
    pub(super) args: &'a [CommandArg],
    /// Index of the argument being emitted.
    pub(super) index: usize,
    /// Argument indices the registry marked `ArgRole::Expr`.
    expr_args: &'a [usize],
    pub(super) config: &'a FormatterConfig,
    indent_level: usize,
    /// Canonical replacement text for this word, if any.
    keyword_rewrite: Option<&'a str>,
}

/// Append a plain word argument (with optional expression wrapping).
/// Returns `false` when nothing was emitted (an all-continuation artifact),
/// so the caller leaves the brace chain untouched.
fn append_word_arg(ctx: &WordArgContext<'_>, parts: &mut Vec<String>) -> bool {
    let WordArgContext {
        sm,
        args,
        index: i,
        expr_args,
        config,
        indent_level,
        keyword_rewrite,
    } = *ctx;
    let arg = &args[i];
    // A keyword rewrite
    // replaces the whole word. It is only ever computed for a plain, static,
    // unbraced, unquoted keyword word, so there are no delimiters to preserve
    // and no expression/body handling to run.
    if let Some(text) = keyword_rewrite {
        if !parts.is_empty() {
            parts.push(" ".to_owned());
        }
        parts.push(text.to_owned());
        return true;
    }
    let mut raw = reconstruct_arg(sm, arg, config.enforce_braced_variables);
    if raw.contains('\n')
        && !arg.is_braced
        && !arg.tokens.iter().any(|token| {
            token.kind == TokenType::Cmd
                || (matches!(token.kind, TokenType::Var | TokenType::ExprSugar)
                    && sm.token_text(*token).contains('\n'))
        })
    {
        // Keep whitespace before the backslash only for a quoted string, where
        // it is literal data; a bare/continued word collapses.
        let collapsed = normalise_backslash_newline(&raw, arg.is_quoted)
            .trim()
            .to_owned();
        if collapsed.is_empty() {
            return false;
        }
        raw = collapsed;
    }
    // `enforceBracedExpr`, bounded form: a single unbraced expression argument
    // (an `if` / `while` / `for` condition, `control::assert`'s first arg) is
    // wrapped in braces (`if $x …` → `if {$x} …`). A `"…"`-quoted operand, or
    // one carrying a `{*}` expansion (whose expansion braces would demote to
    // literal text), is left alone.
    let has_expansion = arg.tokens.iter().any(|t| t.kind == TokenType::Expand);
    if config.enforce_braced_expr
        && !arg.is_braced
        && !arg.is_quoted
        && !has_expansion
        && expr_args.contains(&i)
    {
        maybe_space(parts, false, config.space_between_braces);
        parts.push(format!("{{{raw}}}"));
        return true;
    }
    if arg.is_braced && expr_args.contains(&i) {
        let indent_len = config.make_indent(indent_level).len();
        let mut col = current_col(parts, indent_len);
        if !parts.is_empty() {
            col += 1;
        }
        let trailing = estimate_trailing_len(args, i + 1);
        if col + raw.len() + trailing > config.max_line_length
            && let Some(wrapped) = wrap_braced_expr(&arg.text, config, indent_level)
        {
            maybe_space(parts, false, config.space_between_braces);
            parts.push(format!("{{{wrapped}}}"));
            return true;
        }
    }
    if !parts.is_empty() {
        parts.push(" ".to_owned());
    }
    parts.push(raw);
    true
}

/// The keyword rewrites
/// that apply to this command, keyed by argument index.
///
/// Only plain `Word` arguments that are neither braced nor quoted are
/// eligible: a braced or quoted word's bytes are data the author chose to
/// delimit, and a `Body`/`ParamList`/`Keyword`/`LambdaLiteral` argument is
/// not a keyword-table position at all. Everything else — the dialect gate,
/// the strictness flag, ambiguity, dynamic words — is decided by
/// [`super::super::keywords::rewrites_for_command`].
pub(super) fn keyword_rewrites_for(
    cmd: &ParsedCommand,
    _registry: &CommandRegistry,
    config: &FormatterConfig,
) -> std::collections::HashMap<usize, String> {
    // A source schema identifies possible positions. Operand-byte rewriting
    // requires a separate equivalence receipt; this view supplies none.
    if cmd.source_roles.is_some() || (cmd.logical_roles.is_some() && cmd.logical_context.is_none())
    {
        return std::collections::HashMap::new();
    }
    if !config.expand_abbreviations
        && config.boolean_form == super::super::config::BooleanForm::Preserve
    {
        return std::collections::HashMap::new();
    }
    if cmd.resolved_name.is_empty() {
        return std::collections::HashMap::new();
    }
    let (Some(source), Some(context)) = (&cmd.source_words, &cmd.logical_context) else {
        return std::collections::HashMap::new();
    };
    if source
        .head_source()
        .and_then(|head| head.word())
        .is_none_or(|word| word.config() != config.lexer_config())
    {
        return std::collections::HashMap::new();
    }
    source
        .with_source_schema(context, |selected| {
            let surviving =
                super::super::keywords::rewrites_for_selected_invocation(selected, config)
                    .into_iter()
                    .filter_map(|rewrite| {
                        let tcl_compiler::registry_invocation::InvocationWordOrigin::Written(
                            written,
                        ) = source.origins.get(rewrite.index.checked_add(1)?)?
                        else {
                            return None;
                        };
                        if *written == 0 {
                            return None;
                        }
                        let argument = cmd.args.get(*written)?;
                        if argument.kind != ArgKind::Word
                            || argument.is_braced
                            || argument.is_quoted
                            || argument.tokens.iter().any(|token| {
                                matches!(
                                    token.kind,
                                    TokenType::Var | TokenType::Cmd | TokenType::Expand
                                )
                            })
                            || selected.words.arguments().literal_at(rewrite.index)
                                != Some(argument.text.as_str())
                            || source.operands.get(rewrite.index)?.as_ref()?.word.as_ref()
                                != argument.original_word.as_ref()
                        {
                            return None;
                        }
                        Some((rewrite.index, *written, rewrite.text))
                    })
                    .collect::<Vec<_>>();
            let replacements = surviving
                .iter()
                .map(|(effective, _, text)| (*effective, text.as_str()))
                .collect::<Vec<_>>();
            if !super::super::keywords::emitted_replacements_keep_selection(
                selected,
                config,
                &replacements,
            ) {
                return std::collections::HashMap::new();
            }
            surviving
                .into_iter()
                .map(|(_, written, text)| (written, text))
                .collect()
        })
        .unwrap_or_default()
}

/// Reconstruct a single command as formatted text.
pub(super) fn case_list_body_index(
    cmd: &ParsedCommand,
    config: &FormatterConfig,
    registry: &CommandRegistry,
) -> Option<(usize, CaseListSpec)> {
    if cmd.source_roles.is_some() || cmd.logical_roles.is_some() {
        return cmd.source_case;
    }
    let case_list = registry
        .get(&cmd.resolved_name)
        .and_then(|spec| spec.case_list)?;
    let args: Vec<&str> = cmd
        .args
        .iter()
        .skip(1)
        .map(|arg| arg.text.as_str())
        .collect();
    let dialect = registry.profile().map_or_else(
        || config.dialect_query(),
        |profile| Some(crate::document_context_for_profile(profile).authoring_query()),
    );
    if let Some(index) = registry
        .case_invocation(&cmd.resolved_name, &args, dialect)
        .and_then(|(_, invocation)| invocation.clause_list_index)
    {
        return Some((index + 1, *case_list));
    }

    // Formatting may present an incomplete flag-free (switch-shaped) case
    // list even when semantic consumers correctly abstain. A flagged
    // descriptor cannot recover safely: after an unknown, ambiguous, or
    // truncated flag, later words have no proven pattern/body roles and the
    // shared splitter deliberately resynchronises only for diagnostics. Keep
    // that whole value opaque instead of formatting an apparent later body.
    if !case_list.clause_flags.is_empty() || !case_list.clause_value_flags.is_empty() {
        return None;
    }

    // Prove that the original value is a complete Tcl list with precisely the
    // odd pattern/body prefix shape formatter recovery supports. Empty and
    // otherwise malformed values remain byte-semantically opaque.
    let last = args.len().checked_sub(1)?;
    let elements = config.word_rules().split_list(args[last]).ok()?;
    if elements.is_empty() || elements.len().is_multiple_of(2) {
        return None;
    }

    // Probe the registry-owned *outer* grammar with a known-valid list in the
    // final word. This proves that the original word occupies the clause-list
    // slot without blessing its dangling pattern as executable. Inline
    // pattern/body forms remain inline because replacing their final body does
    // not change their arity.
    let mut recovery_args = args;
    recovery_args[last] = "default {}";
    registry
        .case_invocation(&cmd.resolved_name, &recovery_args, dialect)
        .and_then(|(_, invocation)| {
            (invocation.clause_list_index == Some(last)).then_some((last + 1, *case_list))
        })
}

pub(super) fn reconstruct_command(
    sm: &SourceMap,
    cmd: &ParsedCommand,
    config: &FormatterConfig,
    registry: &CommandRegistry,
    indent: &str,
    indent_level: usize,
) -> String {
    if case_list_body_index(cmd, config, registry).is_some() {
        return reconstruct_case_list(sm, cmd, config, indent);
    }

    let expr_args = identify_expr_args(cmd, registry);
    let mut effective_config = config.clone();
    if cmd.source_roles.is_some() {
        effective_config.enforce_braced_expr = false;
    }
    let config = &effective_config;
    let keyword_rewrites = keyword_rewrites_for(cmd, registry, config);
    let spec_traits = if cmd.source_roles.is_some() || cmd.logical_roles.is_some() {
        cmd.source_traits
    } else {
        registry.get(&cmd.resolved_name).map(|s| s.traits)
    };
    let never_inline = spec_traits.is_some_and(|t| t.contains(Traits::NEVER_INLINE_BODY));

    let mut parts: Vec<String> = Vec::new();
    let mut in_brace_chain = false;

    // `enforceBracedExpr`, concatenating form: a command whose entire argument
    // tail is one expression (`expr $a + $b`) — brace the joined tail
    // (`expr {$a + $b}`) rather than the single marked argument, which would
    // corrupt it. Bounded-expression commands fall through to
    // the per-argument bracing in `append_word_arg`.
    if config.enforce_braced_expr
        && spec_traits.is_some_and(|t| t.contains(Traits::EXPR_CONCATENATES_ARGS))
        && let Some(braced) = concat_expr_parts(sm, cmd, config)
    {
        return finish_command_line(&braced, config, indent, indent_level);
    }

    for (i, arg) in cmd.args.iter().enumerate() {
        match arg.kind {
            ArgKind::Body if arg.formatted_body.is_some() => {
                let body = arg.formatted_body.as_deref().unwrap();
                let current_line_len = indent.len() + parts.concat().len();
                // The body's leading space depends on the running
                // brace chain; apply it here, then delegate the
                // body rendering.
                maybe_space(&mut parts, in_brace_chain, config.space_between_braces);
                append_body_no_space(
                    &mut parts,
                    body,
                    config,
                    indent,
                    current_line_len,
                    never_inline || arg.member_body_presentation == Some(
                        tcl_registry::definer::SourceDefinitionMemberBodyPresentation::CallableBlock,
                    ),
                );
                in_brace_chain = true;
            }
            ArgKind::LambdaLiteral if arg.formatted_body.is_some() => {
                let current_line_len = indent.len() + parts.concat().len();
                maybe_space(&mut parts, in_brace_chain, config.space_between_braces);
                parts.push(render_lambda_literal_arg(
                    sm,
                    arg,
                    config,
                    indent,
                    current_line_len,
                    never_inline,
                    cmd.logical_context.is_some(),
                ));
                in_brace_chain = true;
            }
            ArgKind::ParamList => {
                maybe_space(&mut parts, false, config.space_between_braces);
                parts.push(render_param_list_arg(
                    sm,
                    arg,
                    config,
                    cmd.source_roles.is_none(),
                ));
                in_brace_chain = false;
            }
            ArgKind::Keyword => {
                maybe_space(&mut parts, false, config.space_between_braces);
                parts.push(arg.text.clone());
                in_brace_chain = false;
            }
            _ => {
                if let Some((written, replacement)) = &cmd.expression_bracing
                    && *written == i
                {
                    maybe_space(&mut parts, false, config.space_between_braces);
                    parts.push(replacement.clone());
                    in_brace_chain = false;
                    continue;
                }
                if append_word_arg(
                    &WordArgContext {
                        sm,
                        args: &cmd.args,
                        index: i,
                        expr_args: &expr_args,
                        config,
                        indent_level,
                        keyword_rewrite: keyword_rewrites.get(&i).map(String::as_str),
                    },
                    &mut parts,
                ) {
                    in_brace_chain = false;
                }
            }
        }
    }

    finish_command_line(&parts, config, indent, indent_level)
}

/// Concatenate `parts` under `indent` and apply the long-line split. Shared by
/// the normal command path and the `enforceBracedExpr` concatenating path.
fn finish_command_line(
    parts: &[String],
    config: &FormatterConfig,
    indent: &str,
    indent_level: usize,
) -> String {
    let line = format!("{indent}{}", parts.concat());

    // Split the first line if it exceeds max_line_length.
    let first_nl = line.find('\n');
    let first_line = match first_nl {
        Some(nl) => &line[..nl],
        None => &line,
    };
    if first_line.len() > config.max_line_length {
        let cont_indent = config.make_indent(indent_level + 1);
        if let Some(split) = split_long_line(first_line, config, &cont_indent) {
            return match first_nl {
                Some(nl) => format!("{split}{}", &line[nl..]),
                None => split,
            };
        }
    }
    line
}

/// Build the `parts` for an `enforceBracedExpr` command whose entire argument
/// tail is one expression (`EXPR_CONCATENATES_ARGS`). Returns `None` when there
/// is nothing to brace — no arguments, or the tail is already a single braced
/// `{ … }` word (which the normal path renders, keeping its long-line wrap).
fn concat_expr_parts(
    sm: &SourceMap,
    cmd: &ParsedCommand,
    config: &FormatterConfig,
) -> Option<Vec<String>> {
    let args = &cmd.args;
    if args.len() < 2 {
        return None; // bare `expr` with no operands
    }
    if args.len() == 2 && args[1].is_braced {
        return None; // already `expr {…}`
    }
    // Never brace a tail containing an expansion: `expr {*}$pieces` expands the
    // list before evaluating, and `expr {{*}$pieces}` would demote `{*}` to
    // literal text, breaking the expression. Leave such a command untouched.
    if args[1..]
        .iter()
        .any(|a| a.tokens.iter().any(|t| t.kind == TokenType::Expand))
    {
        return None;
    }
    let joined = args[1..]
        .iter()
        .map(|a| reconstruct_arg(sm, a, config.enforce_braced_variables))
        .collect::<Vec<_>>()
        .join(" ");
    Some(vec![
        args[0].text.clone(),
        " ".to_owned(),
        format!("{{{joined}}}"),
    ])
}

/// Reconstruct a `switch` command, handling the braced body form.
fn reconstruct_case_list(
    sm: &SourceMap,
    cmd: &ParsedCommand,
    config: &FormatterConfig,
    indent: &str,
) -> String {
    // The case-list body is the last braced arg; recompute its
    // formatting through `format_case_list_body` (parsed below by
    // re-deriving from the registry-marked Body arg or the last
    // braced arg).
    let mut parts: Vec<String> = Vec::new();
    for arg in &cmd.args {
        if arg.kind == ArgKind::Body && arg.is_braced {
            let formatted = arg.formatted_body.clone().unwrap_or_default();
            if !parts.is_empty() {
                parts.push(" ".to_owned());
            }
            if formatted.trim().is_empty() {
                parts.push("{}".to_owned());
            } else {
                parts.push("{\n".to_owned());
                parts.push(formatted);
                parts.push(format!("\n{indent}}}"));
            }
        } else {
            let raw = reconstruct_arg(sm, arg, config.enforce_braced_variables);
            if !parts.is_empty() {
                parts.push(" ".to_owned());
            }
            parts.push(raw);
        }
    }
    format!("{indent}{}", parts.concat())
}
