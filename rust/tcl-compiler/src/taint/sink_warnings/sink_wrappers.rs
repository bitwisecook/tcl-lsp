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

//! Wrappers around a tainted variable in a sink argument: literal `list` commands, sanitisers and the colours transforms stamp.

use super::super::{
    CommandRegistry, CommandTokens, SinkCall, TaintColour, TaintSourceContext, WordExpr, WordPart,
    arg_var_names_with_config, is_sanitiser, parse_command_substitution_with_config, reg_colour,
    transform_colour,
};

pub(in super::super) fn standalone_list_wrapper_head(
    argument: &str,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
    dialect: Option<tcl_native_model::InvocationDialect>,
) -> Option<String> {
    let original = argument.trim();
    if original.as_bytes().first() != Some(&b'[') {
        return None;
    }
    let end = tcl_lexer::command_subst_close(
        original.as_bytes(),
        0,
        tcl_lexer::SubstFlags::default(),
        config,
    )
    .ok()?;
    if end != original.len() {
        return None;
    }
    let image = tcl_lexer::SourceImage::document(original);
    let plan = tcl_lexer::native_script_words_in_with_purpose(
        image,
        tcl_lexer::Span::new(1, u32::try_from(end.checked_sub(1)?).ok()?),
        config.nested(),
        tcl_lexer::SourceSegmentationPurpose::NestedScript,
    )
    .ok()?;
    let [command] = plan.commands.as_slice() else {
        return None;
    };
    if plan.fatal_tail.is_some() {
        return None;
    }
    let invocation = crate::registry_invocation::standalone_written_source_name_roles(
        registry,
        &command.words,
        dialect,
    )?;
    if invocation.has_unknown_residual()
        || !matches!(
            invocation.facts().native_result,
            Some(tcl_registry::native_result::NativeResultContract::ListArguments { from: 0 })
        )
    {
        return None;
    }
    let head = invocation.argument_literal(0)?;
    (!head.contains('\0') && registry.get(&head).is_some()).then_some(head)
}

fn single_script_operand(call: &SinkCall<'_>, argument: usize) -> bool {
    let Some(expected) = call.argument_offset.checked_add(argument) else {
        return false;
    };
    if let Some(scripts) = call
        .source_read
        .and_then(|read| read.script_arguments(call))
    {
        return scripts == [expected];
    }
    let (Some(metadata), Some(tokens), Some(effective)) =
        (call.source.metadata, call.tokens, call.effective)
    else {
        return false;
    };
    crate::registry_invocation::actual_taint_script_arguments(
        call.registry,
        metadata,
        tokens,
        call.command,
        effective,
    )
    .is_some_and(|scripts| scripts == [expected])
}

/// `true` when one of `args` is a `[list <head> …]` command substitution
/// whose constructed-list command word (`<head>`) is a literal known
/// registry command and `name` is referenced in that argument. The tainted
/// variable is then a *quoted argument* of the list, never the command
/// word, so `eval`/`uplevel`/`interp eval` of the list runs no injected
/// command. `[list $x …]` (variable head) and `[list]`-free args return
/// `false`.
pub(in super::super) fn list_wrapped_arg_command_is_literal(
    call: &SinkCall<'_>,
    name: &str,
) -> bool {
    let (registry, args, config) = (call.registry, call.args, call.source.config);
    if args
        .iter()
        .any(|arg| arg_var_names_with_config(arg, config).has_unknown_residual())
    {
        return false;
    }
    for (index, arg) in args.iter().enumerate() {
        let trimmed = arg.trim();
        if let Some(tokens) = call.tokens
            && tokens.source_binding.is_some()
        {
            if !arg_var_names_with_config(arg, config).may_contain(name) {
                continue;
            }
            // Tcl concatenation can expose quoted list text to another
            // script operand's syntax. Both old reached-wrapper and new CPP
            // proofs therefore require this one complete script operand.
            if !single_script_operand(call, index) {
                continue;
            }
            let quoted_list = call.source.metadata.is_some_and(|metadata| {
                retained_wrapper_tokens(
                    tokens,
                    call.argument_offset + index,
                    trimmed,
                    call.effective,
                    call.source,
                )
                .is_some_and(|nested| {
                    !nested.is_empty()
                        && nested.iter().all(|nested| {
                            crate::registry_invocation::original_compiled_list_head(
                                registry, metadata, tokens, nested,
                            )
                            .or_else(|| {
                                crate::registry_invocation::original_conditional_list_result_head(
                                    registry, metadata, tokens, nested,
                                )
                            })
                            .is_some()
                        })
                })
            });
            if quoted_list {
                return true;
            }
            let Some(invocations) = proved_wrapper_invocations(
                registry,
                tokens,
                call.argument_offset + index,
                trimmed,
                call.effective,
                call.source,
            ) else {
                continue;
            };
            if invocations.iter().all(|invocation| {
                matches!(
                    invocation.facts.native_result,
                    Some(
                        tcl_registry::native_result::NativeResultContract::ListArguments {
                            from: 0
                        }
                    )
                ) && invocation
                    .argument_literal(0)
                    .is_some_and(|head| registry.get(&head).is_some())
            }) {
                return true;
            }
            continue;
        }
        if !call.source.allows_nominal_metadata() {
            continue;
        }
        if standalone_list_wrapper_head(
            trimmed,
            registry,
            config,
            call.source
                .dialect()
                .map(tcl_native_model::InvocationDialect::of_profile),
        )
        .is_none()
        {
            continue;
        }
        // `name` must actually flow through this list arg (and, since the
        // head is a literal, only at an argument position ≥ 1).
        if arg_var_names_with_config(arg, config).may_contain(name) {
            return true;
        }
    }
    false
}

/// Split `arg` into its residual text (everything outside the command
/// substitutions it really performs) and the list of those `[...]` slices.
///
/// Segmentation is the lexer's, not a bracket count, because a `[` is only an
/// opener when the grammar says so. `regexp -- "\[regex::quote $p]" $s` writes
/// an *escaped* bracket: tclsh 8.6.18 and 9.0.4 both leave it as literal text
/// and substitute `$p` straight into the pattern, never calling the quoter. A
/// counting scan reads a command substitution there and hands callers a
/// wrapper that does not run — which, for the mitigation callers, means
/// crediting a sanitiser the value never passed through.
///
/// The parts come back with their source extents, so a returned slice is the
/// `[...]` text a caller can re-parse. A part the lexer reports as a variable
/// or literal run is residual: only a real `Command` is a wrapper.
fn split_top_level_cmd_subs(arg: &str, config: tcl_lexer::LexerConfig) -> (String, Vec<&str>) {
    let flags = tcl_lexer::word_parts::SubstFlags::default();
    let Ok(length) = u32::try_from(arg.len()) else {
        return (arg.to_owned(), Vec::new());
    };
    let Ok(arena) = tcl_lexer::word_parts::ExecutablePartArena::decompose(
        tcl_lexer::SourceImage::native(arg.as_bytes()),
        tcl_lexer::Span::new(0, length),
        flags,
        config,
    ) else {
        return (arg.to_owned(), Vec::new());
    };
    if arena.all_parts().any(|part| {
        matches!(
            part.part,
            tcl_lexer::word_parts::ExecutablePart::ParseError(_)
        )
    }) {
        return (arg.to_owned(), Vec::new());
    }
    let mut residual = String::new();
    let mut subs = Vec::new();
    for part in arena.list(arena.root()) {
        let Some(slice) = arg.get(part.span.as_range()) else {
            return (arg.to_owned(), Vec::new());
        };
        if matches!(
            part.part,
            tcl_lexer::word_parts::ExecutablePart::Command { .. }
        ) {
            subs.push(slice);
        } else {
            residual.push_str(slice);
        }
    }
    (residual, subs)
}

/// Resolve every exact source occurrence of a wrapper in this effective word.
/// Identical spellings can have different implementations after an earlier
/// substitution; all occurrences must independently establish their facts.
fn retained_wrapper_tokens(
    tokens: &CommandTokens,
    argument: usize,
    spelling: &str,
    projection: Option<&crate::registry_invocation::EffectiveCommandWords>,
    context: TaintSourceContext<'_>,
) -> Option<Vec<CommandTokens>> {
    let recovered = crate::registry_invocation::effective_command_words(tokens);
    let effective = projection.or(recovered.as_ref())?;
    let word = effective.words.get(argument.checked_add(1)?)?;
    let mut sources = Vec::new();
    match word {
        WordExpr::CommandSubstitution {
            spelling: text,
            source,
        } if text == spelling => {
            sources.push(source);
        }
        WordExpr::Template { parts, .. } => {
            for part in parts {
                if let WordPart::CommandSubstitution {
                    spelling: text,
                    source,
                } = part
                    && text == spelling
                {
                    sources.push(source);
                }
            }
        }
        _ => return None,
    }
    if sources.is_empty() {
        return None;
    }
    let config = if context.allows_nominal_metadata() {
        let dialect = tokens
            .source_binding
            .as_ref()?
            .variable_context
            .invocation_dialect?;
        tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar)
    } else {
        context.metadata?;
        context.config
    };
    sources
        .into_iter()
        .map(|source| {
            let mut nested =
                crate::word_subst::nested_command_words(spelling, source, config).ok()?;
            nested.inherit_nested_bindings(tokens);
            Some(nested)
        })
        .collect()
}

fn proved_wrapper_invocations(
    registry: &CommandRegistry,
    tokens: &CommandTokens,
    argument: usize,
    spelling: &str,
    projection: Option<&crate::registry_invocation::EffectiveCommandWords>,
    context: TaintSourceContext<'_>,
) -> Option<Vec<crate::registry_invocation::ResolvedStatementInvocation>> {
    retained_wrapper_tokens(tokens, argument, spelling, projection, context)?
        .iter()
        .map(|nested| {
            crate::registry_invocation::resolved_tokens_invocation_with_metadata_context(
                registry,
                context.metadata,
                nested,
            )
        })
        .collect()
}

fn normal_wrapper_taint_invocations(
    registry: &CommandRegistry,
    tokens: &CommandTokens,
    argument: usize,
    spelling: &str,
    projection: Option<&crate::registry_invocation::EffectiveCommandWords>,
    context: TaintSourceContext<'_>,
) -> Option<Vec<crate::registry_invocation::NormalTaintInvocation>> {
    retained_wrapper_tokens(tokens, argument, spelling, projection, context)?
        .iter()
        .map(|nested| {
            if context.allows_nominal_metadata() && context.metadata.is_none() {
                crate::registry_invocation::normal_taint_invocation(registry, None, nested)
            } else {
                crate::registry_invocation::normal_taint_invocation_with_metadata_context(
                    registry,
                    context.metadata,
                    nested,
                )
            }
        })
        .collect()
}

/// Each occurrence of a handler's wrapper as its hosted command and written
/// arguments: advice for this diagnostic alone, with no handler target or
/// dispatch. `None` when any occurrence has no hosted row.
fn hosted_wrapper_invocations(
    registry: &CommandRegistry,
    tokens: &CommandTokens,
    argument: usize,
    spelling: &str,
    projection: Option<&crate::registry_invocation::EffectiveCommandWords>,
    context: TaintSourceContext<'_>,
) -> Option<Vec<(String, Vec<String>)>> {
    context.metadata?;
    let outer = projection
        .is_none()
        .then(|| crate::registry_invocation::hosted_source_taint_invocation(registry, tokens))
        .flatten();
    let projection = projection.or_else(|| outer.as_ref().map(|outer| &outer.effective));
    retained_wrapper_tokens(tokens, argument, spelling, projection, context)?
        .iter()
        .map(|nested| {
            let hosted =
                crate::registry_invocation::hosted_source_taint_invocation(registry, nested)?;
            // The raw transform condition trusts a word's text only with its
            // written braces, as the statement-level reading does.
            let arguments = nested
                .argv_texts
                .get(1..)?
                .iter()
                .enumerate()
                .map(|(index, spelling)| match nested.word_exprs.get(index + 1) {
                    Some(tcl_source::words::WordExpr::BracedLiteral { text, .. }) => {
                        format!("{{{text}}}")
                    }
                    _ => spelling.clone(),
                })
                .collect();
            Some((hosted.metadata.shape().command().to_owned(), arguments))
        })
        .collect()
}

fn hosted_wrappers_are_sanitisers(
    call: &SinkCall<'_>,
    tokens: &CommandTokens,
    index: usize,
    spelling: &str,
) -> bool {
    hosted_wrapper_invocations(
        call.registry,
        tokens,
        call.argument_offset + index,
        spelling,
        call.effective,
        call.source,
    )
    .is_some_and(|invocations| {
        invocations.iter().all(|(command, args)| {
            let refs: Vec<&str> = args.iter().map(String::as_str).collect();
            is_sanitiser(call.registry, command, &refs)
        })
    })
}

/// True when every appearance of `name` in the sink arguments is consumed by an
/// embedded sanitiser command substitution, so the value reaching the sink is
/// clean — e.g. `puts [string length $x]` outputs the integer length, never
/// `$x`'s content (tclsh-verified). Conservative: a bare `$name` outside any
/// `[...]`, or one inside a *non*-sanitiser substitution, returns `false` (the
/// taint reaches the sink). Mirrors the carve-out the `expr`/`word_taint` path
/// already applies, which `emit_sink_warnings` (iterating raw SSA uses) lacked.
pub(in super::super) fn var_consumed_by_sanitiser(call: &SinkCall<'_>, name: &str) -> bool {
    let (registry, args, config) = (call.registry, call.args, call.source.config);
    if args
        .iter()
        .any(|arg| arg_var_names_with_config(arg, config).has_unknown_residual())
    {
        return false;
    }
    let mut seen = false;
    for (index, arg) in args.iter().enumerate() {
        if !arg_var_names_with_config(arg, config).may_contain(name) {
            continue;
        }
        seen = true;
        let (residual, subs) = split_top_level_cmd_subs(arg, config);
        // A bare reference outside any command substitution reaches the sink.
        if arg_var_names_with_config(&residual, config).may_contain(name) {
            return false;
        }
        // Each top-level substitution that references `name` must be a sanitiser
        // (a sanitiser's result is a clean bounded value regardless of nesting).
        for sub in subs {
            if !arg_var_names_with_config(sub, config).may_contain(name) {
                continue;
            }
            let Some((cmd, sub_args)) = parse_command_substitution_with_config(sub, config) else {
                return false;
            };
            if let Some(tokens) = call.tokens
                && tokens.source_binding.is_some()
            {
                let Some(invocations) = normal_wrapper_taint_invocations(
                    registry,
                    tokens,
                    call.argument_offset + index,
                    sub,
                    call.effective,
                    call.source,
                ) else {
                    if hosted_wrappers_are_sanitisers(call, tokens, index, sub) {
                        continue;
                    }
                    return false;
                };
                if !invocations
                    .iter()
                    .all(crate::registry_invocation::NormalTaintInvocation::is_sanitiser)
                {
                    return false;
                }
                continue;
            }
            let refs: Vec<&str> = sub_args.iter().map(String::as_str).collect();
            if !call.source.allows_nominal_metadata() || !is_sanitiser(registry, &cmd, &refs) {
                return false;
            }
        }
    }
    seen
}

/// The colours the wrappers around `name` stamp on what actually reaches the
/// sink, when every occurrence of `name` in `args` sits inside a top-level
/// command substitution that declares a `taint_transform`.
///
/// `args` is the slice of the call the diagnostic is judging, which is not
/// always the whole call: a hazard confined to one argument position — a regex
/// pattern, say — is answered by that position alone, and a bare mention of
/// the same variable in a harmless slot must not withdraw the wrapper's
/// proof.
///
/// `log local0. "u=[URI::encode $u]"` and `set e [URI::encode $u]; log local0.
/// "u=$e"` run the same command over the same value and must reach the same
/// verdict. The via-variable spelling gets there through [`word_taint_at`],
/// which stamps the transform colour on the value it assigns; a sink argument
/// is assigned to nothing, so the same fact is recovered here from the call.
/// Wrapping in place is the shape every taint quick fix in
/// `tcl_lsp_core::code_actions` writes, so its colour has to be visible here
/// for the fix it offers to clear the finding it offers it for.
///
/// `None` when any occurrence of `name` reaches the sink unwrapped — the
/// variable's own value arrives, so its own lattice is the answer.
///
/// `Some` means every occurrence is consumed by a wrapper, so what arrives is
/// the wrappers' result and the variable's own proofs say nothing about it.
/// The colours are the intersection across those wrappers, the same rule the
/// lattice uses to join control flow: the value is a concatenation of every
/// part, so a colour survives only where each part proves it. A wrapper that
/// declares no transform, or one this pass cannot parse, therefore yields
/// `Some(empty)` — the value is still tainted and now proves nothing, which is
/// what lets a re-introducing wrapper (`string map {"|" "\n"}` over a
/// CR/LF-free value) reach its sink.
pub(in super::super) fn var_wrapper_colours(
    registry: &CommandRegistry,
    source: TaintSourceContext<'_>,
    args: &[String],
    name: &str,
    tokens: Option<&CommandTokens>,
    argument_offset: usize,
) -> Option<TaintColour> {
    let config = source.config;
    if args
        .iter()
        .any(|arg| arg_var_names_with_config(arg, config).has_unknown_residual())
    {
        return None;
    }
    let mut colours = TaintColour::all();
    let mut wrapped = false;
    for (index, arg) in args.iter().enumerate() {
        if !arg_var_names_with_config(arg, config).may_contain(name) {
            continue;
        }
        let (residual, subs) = split_top_level_cmd_subs(arg, config);
        // A bare reference outside any substitution reaches the sink as-is.
        if arg_var_names_with_config(&residual, config).may_contain(name) {
            return None;
        }
        for sub in subs {
            if !arg_var_names_with_config(sub, config).may_contain(name) {
                continue;
            }
            wrapped = true;
            if let Some(tokens) = tokens
                && tokens.source_binding.is_some()
            {
                let colour = normal_wrapper_taint_invocations(
                    registry,
                    tokens,
                    argument_offset + index,
                    sub,
                    None,
                    source,
                )
                .map_or_else(
                    || {
                        hosted_wrapper_invocations(
                            registry,
                            tokens,
                            argument_offset + index,
                            sub,
                            None,
                            source,
                        )
                        .map_or_else(TaintColour::empty, |invocations| {
                            invocations.iter().fold(
                                TaintColour::all(),
                                |colours, (command, args)| {
                                    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
                                    colours
                                        & transform_colour(registry, command, &refs)
                                            .unwrap_or_else(TaintColour::empty)
                                },
                            )
                        })
                    },
                    |invocations| {
                        invocations
                            .iter()
                            .fold(TaintColour::all(), |colours, invocation| {
                                colours
                                    & invocation
                                        .transform_colour()
                                        .map_or_else(TaintColour::empty, reg_colour)
                            })
                    },
                );
                colours &= colour;
                continue;
            }
            let colour = source
                .allows_nominal_metadata()
                .then_some(())
                .and_then(|()| parse_command_substitution_with_config(sub, config))
                .and_then(|(cmd, sub_args)| {
                    let refs: Vec<&str> = sub_args.iter().map(String::as_str).collect();
                    transform_colour(registry, &cmd, &refs)
                })
                .unwrap_or_else(TaintColour::empty);
            colours &= colour;
        }
    }
    wrapped.then_some(colours)
}
