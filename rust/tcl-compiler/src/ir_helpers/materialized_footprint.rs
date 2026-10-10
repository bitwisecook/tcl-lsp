// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Possible output names from text under an independently retained source world.

use super::{CommandWord, ExecutionNamespace, VariableWriteEffects};
use crate::command_binding::ModuleCommandBindings;
use crate::registry_invocation::InvocationMetadataContext;
use tcl_lexer::{LexerConfig, SourceMap};
use tcl_registry::{ArgRole, BodyInterpreter, BodyKind, CommandRegistry, ScriptTiming, Traits};

struct FootprintContext<'a> {
    registry: &'a CommandRegistry,
    metadata: InvocationMetadataContext<'a>,
    namespace: &'a ExecutionNamespace,
    config: LexerConfig,
}

impl VariableWriteEffects {
    pub(crate) fn include_possible(&mut self, other: &Self) {
        self.opaque |= other.opaque;
        for (found, names) in [
            (&other.names, &mut self.names),
            (&other.read_names, &mut self.read_names),
        ] {
            for name in found {
                let root = tcl_syntax::naming::split_array_name_braced(name, true).0;
                if !root.is_empty() && !names.iter().any(|name| name == root) {
                    names.push(root.to_owned());
                }
            }
        }
    }
}

/// Typed possible writes under the retained command world and full availability.
/// Materialized text is value syntax, never an original child invocation.
pub(crate) fn script_value_possible_writes_with_metadata_context(
    text: &str,
    registry: &CommandRegistry,
    bindings: &ModuleCommandBindings,
    namespace: &ExecutionNamespace,
    metadata: Option<InvocationMetadataContext<'_>>,
    config: LexerConfig,
) -> VariableWriteEffects {
    let Some(metadata) = metadata.filter(|metadata| metadata.matches_registry(registry)) else {
        return VariableWriteEffects {
            opaque: true,
            ..VariableWriteEffects::default()
        };
    };
    let context = FootprintContext {
        registry,
        metadata,
        namespace,
        config,
    };
    let mut state = bindings.clone();
    let mut out = VariableWriteEffects::default();
    script_writes(text, &context, &mut state, &mut out, 0);
    out
}

/// Analytical expression footprint using the same selected source world.
/// Conditional expression branches contribute possible names only.
pub(crate) fn expression_possible_writes_with_metadata_context(
    expression: &crate::expr_ast::ExprNode,
    registry: &CommandRegistry,
    bindings: &ModuleCommandBindings,
    namespace: &ExecutionNamespace,
    metadata: Option<InvocationMetadataContext<'_>>,
    config: LexerConfig,
) -> VariableWriteEffects {
    let Some(metadata) = metadata.filter(|metadata| metadata.matches_registry(registry)) else {
        return VariableWriteEffects {
            opaque: true,
            ..VariableWriteEffects::default()
        };
    };
    let context = FootprintContext {
        registry,
        metadata,
        namespace,
        config,
    };
    let mut texts = Vec::new();
    let mut unknown = false;
    let mut conditional = false;
    super::collect_expr_command_surface_refs(
        expression,
        &mut texts,
        &mut unknown,
        &mut conditional,
        0,
    );
    let mut out = VariableWriteEffects {
        opaque: unknown,
        ..VariableWriteEffects::default()
    };
    let mut state = bindings.clone();
    for text in texts {
        let Some(inner) = text
            .strip_prefix('[')
            .and_then(|text| text.strip_suffix(']'))
        else {
            out.opaque = true;
            continue;
        };
        let before = conditional.then(|| state.clone());
        script_writes(inner, &context, &mut state, &mut out, 0);
        if let Some(before) = before {
            state.join_possible_source_effects(&before);
        }
    }
    out
}

fn value_commands(text: &str, config: LexerConfig) -> Vec<Vec<CommandWord>> {
    let source = SourceMap::new(text);
    crate::segmenter::segment_commands_with_offset_and_config(text, 0, config)
        .iter()
        .map(|command| {
            let tokens = crate::ir::CommandTokens::from_segmented(&source, config, command);
            let rules = tcl_syntax::word_rules::WordValueRules::from_config(&config);
            super::command_words(&source, config, command)
                .into_iter()
                .zip(tokens.words())
                .map(|(mut word, original)| {
                    word.raw = source.text(original.source().span).to_owned();
                    let value = crate::registry_invocation::effective_invocation_word(
                        original,
                        config.escapes,
                        rules,
                    );
                    if let Some(value) = value.as_registry_word().literal() {
                        word.text = value.to_owned();
                        word.substituted = false;
                    }
                    word
                })
                .collect()
        })
        .collect()
}

fn script_writes(
    text: &str,
    context: &FootprintContext<'_>,
    state: &mut ModuleCommandBindings,
    out: &mut VariableWriteEffects,
    depth: u32,
) {
    if crate::depth_guard::MAX_BRACKET_TEXT_DEPTH.exceeded(depth) {
        out.opaque = true;
        return;
    }
    for words in value_commands(text, context.config) {
        for word in words.iter().filter(|word| !word.braced_literal) {
            let lexer = tcl_lexer::Lexer::with_config(&word.raw, context.config);
            let map = SourceMap::new(&word.raw);
            for token in lexer {
                let Ok(token) = token else {
                    out.opaque = true;
                    continue;
                };
                if token.kind == tcl_lexer::TokenType::Cmd {
                    script_writes(map.token_text(token), context, state, out, depth + 1);
                }
            }
        }
        command_writes(&words, context, state, out, depth);
    }
}

fn command_writes(
    words: &[CommandWord],
    context: &FootprintContext<'_>,
    state: &mut ModuleCommandBindings,
    out: &mut VariableWriteEffects,
    depth: u32,
) {
    let Some(holder) = words
        .first()
        .and_then(CommandWord::literal)
        .and_then(|head| context.namespace.for_head_context(head))
    else {
        out.opaque = true;
        return;
    };
    let projection = state.variable_write_projection_for_command_words_with_metadata_context(
        words,
        context.registry,
        Some(context.metadata),
        holder.as_ref(),
    );
    out.include_possible(&VariableWriteEffects {
        names: projection.literal_names,
        read_names: projection.read_before_write_names,
        opaque: projection.opaque_variable_frame,
    });
    let reads = state.variable_read_projection_for_command_words_with_metadata_context(
        words,
        context.registry,
        Some(context.metadata),
        holder.as_ref(),
    );
    out.include_possible(&VariableWriteEffects {
        read_names: reads.literal_names,
        opaque: reads.opaque_variable_frame,
        ..VariableWriteEffects::default()
    });
    let bodies = immediate_bodies(words, context, state, holder.as_ref());
    for body in bodies {
        let mut branch = state.clone();
        script_writes(&body, context, &mut branch, out, depth + 1);
        state.join_possible_source_effects(&branch);
    }
    state.source_order_registry_barrier_for_command_with_metadata_context(
        words,
        false,
        context.registry,
        context.namespace,
        Traits::EVALUATES_CODE,
        Some(context.metadata),
    );
}

fn immediate_bodies(
    words: &[CommandWord],
    context: &FootprintContext<'_>,
    state: &ModuleCommandBindings,
    holder: &crate::command_binding::SourceNamespaceKey,
) -> Vec<String> {
    let mut bodies = Vec::new();
    state.for_each_resolved_command_words(words, holder, |target, invocation| {
        if !target.registry_backed {
            return;
        }
        let resolution =
            tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
                context.registry,
                Some(context.metadata.context()),
                invocation,
                state.invocation_realm(),
            );
        let Some(schema) = resolution.resolved() else {
            return;
        };
        bodies.extend(immediate_same_frame_script_values(&schema));
    });
    bodies
}

/// Static values in immediate current-frame bodies of an already selected
/// descriptor. This is possible source evaluation, never entered-script proof.
pub(crate) fn immediate_same_frame_script_values(
    schema: &tcl_registry::ResolvedInvocation<'_, '_>,
) -> Vec<String> {
    if schema.semantics.body_kind != BodyKind::Plain
        || schema.semantics.body_interpreter != BodyInterpreter::Current
    {
        return Vec::new();
    }
    let (roles, complete) = schema.authored_source_argument_roles();
    if !complete {
        return Vec::new();
    }
    let arguments = schema.words.arguments();
    let mut bodies = Vec::new();
    for (argument, role) in roles {
        let argument = usize::from(argument) + schema.semantics.argument_offset;
        if role != ArgRole::Body
            || schema.authored_source_script_timing_at(argument)
                != Some(ScriptTiming::SameInvocation)
        {
            continue;
        }
        let value = if schema
            .semantics
            .traits
            .contains(Traits::SCRIPT_CONCATENATES_ARGS)
        {
            let Some(values) = (argument..arguments.len())
                .map(|ordinal| arguments.literal_at(ordinal))
                .collect::<Option<Vec<_>>>()
            else {
                continue;
            };
            match arguments
                .dialect()
                .and_then(|dialect| dialect.native_family)
            {
                Some(tcl_dialect::model::Family::Jim) => {
                    tcl_syntax::list::concat_values_jim(values)
                }
                Some(
                    tcl_dialect::model::Family::Tcl
                    | tcl_dialect::model::Family::F5Tcl
                    | tcl_dialect::model::Family::F5Irules,
                ) => tcl_syntax::list::concat_values(values),
                _ => continue,
            }
        } else {
            let Some(value) = arguments.literal_at(argument) else {
                continue;
            };
            value.to_owned()
        };
        bodies.push(value);
    }
    bodies
}
