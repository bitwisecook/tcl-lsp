// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Possible output names from text under an independently retained source world.

use super::{CommandWord, ExecutionNamespace, VariableWriteEffects};
use crate::command_binding::ModuleCommandBindings;
use crate::registry_invocation::InvocationMetadataContext;
use tcl_lexer::{LexerConfig, SourceMap};
use tcl_registry::{ArgRole, BodyKind, CommandRegistry, ScriptTiming, Traits};

#[derive(Clone, Copy, PartialEq, Eq)]
enum ReadPurpose {
    InterpolationAndNames,
    NamesOnly,
}

struct FootprintContext<'a> {
    registry: &'a CommandRegistry,
    metadata: InvocationMetadataContext<'a>,
    namespace: &'a ExecutionNamespace,
    config: LexerConfig,
    reads: ReadPurpose,
    ownership: Option<crate::script_binds::Ownership>,
}

impl VariableWriteEffects {
    pub(crate) fn include_possible(&mut self, other: &Self) {
        self.opaque |= other.opaque;
        for (found, names) in [
            (&other.names, &mut self.names),
            (&other.read_names, &mut self.read_names),
        ] {
            for name in found {
                let root = tcl_syntax::naming::split_element_ref(name)
                    .map_or(name.as_str(), |(root, _)| root);
                if !names.iter().any(|name| name == root) {
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
        reads: ReadPurpose::InterpolationAndNames,
        ownership: None,
    };
    let mut state = bindings.clone();
    let mut out = VariableWriteEffects::default();
    script_writes(text, &context, &mut state, &mut out, 0);
    out
}

/// Possible output and by-name input operands of a materialized value.
/// Interpolation reads stay separate: a body cannot own a missing name merely
/// because that same body contains `$name`. No child command receipt is issued.
pub(crate) fn script_value_name_ownership_with_metadata_context(
    text: &str,
    registry: &CommandRegistry,
    bindings: &ModuleCommandBindings,
    namespace: &ExecutionNamespace,
    metadata: InvocationMetadataContext<'_>,
    config: LexerConfig,
    purpose: crate::script_binds::Ownership,
) -> VariableWriteEffects {
    if !metadata.matches_registry(registry) {
        return VariableWriteEffects {
            opaque: true,
            ..Default::default()
        };
    }
    let context = FootprintContext {
        registry,
        metadata,
        namespace,
        config,
        reads: ReadPurpose::NamesOnly,
        ownership: Some(purpose),
    };
    let mut state = bindings.clone();
    let mut out = VariableWriteEffects::default();
    script_writes(text, &context, &mut state, &mut out, 0);
    out
}

/// Conditional names in the selected original command and its re-evaluated
/// values, using the same closed source lookup as materialized script advice.
pub(crate) fn command_possible_footprint_with_metadata_context(
    words: &[CommandWord],
    registry: &CommandRegistry,
    bindings: &ModuleCommandBindings,
    namespace: &ExecutionNamespace,
    metadata: InvocationMetadataContext<'_>,
    config: LexerConfig,
) -> VariableWriteEffects {
    let context = FootprintContext {
        registry,
        metadata,
        namespace,
        config,
        reads: ReadPurpose::InterpolationAndNames,
        ownership: None,
    };
    let mut state = bindings.clone();
    let mut out = VariableWriteEffects::default();
    command_writes(words, &context, &mut state, &mut out, 0);
    out
}

/// Possible named reads at one independently retained original dispatch.
/// Read/write metadata and re-evaluated values remain conditional source facts.
pub(crate) fn command_possible_reads_with_metadata_context(
    words: &[CommandWord],
    registry: &CommandRegistry,
    bindings: &ModuleCommandBindings,
    namespace: &ExecutionNamespace,
    metadata: InvocationMetadataContext<'_>,
    config: LexerConfig,
    include_invocation: bool,
) -> VariableWriteEffects {
    let context = FootprintContext {
        registry,
        metadata,
        namespace,
        config,
        reads: ReadPurpose::InterpolationAndNames,
        ownership: None,
    };
    let mut state = bindings.clone();
    let mut out = VariableWriteEffects::default();
    if include_invocation {
        command_writes(words, &context, &mut state, &mut out, 0);
    } else {
        let Some(holder) = words
            .first()
            .and_then(CommandWord::literal)
            .and_then(|head| namespace.for_head_context(head))
        else {
            return VariableWriteEffects {
                opaque: true,
                ..Default::default()
            };
        };
        reevaluated_reads(words, &context, &mut state, holder.as_ref(), &mut out, 0);
    }
    out.names.clear();
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
        reads: ReadPurpose::InterpolationAndNames,
        ownership: None,
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

pub(crate) fn footprint_command_words(
    source: &SourceMap<'_>,
    config: LexerConfig,
    command: &crate::segmenter::SegmentedCommand,
) -> Vec<CommandWord> {
    let words = super::command_words(source, config, command);
    if words.iter().any(|word| !word.source_available) {
        return words;
    }
    let tokens = crate::ir::CommandTokens::from_segmented(source, config, command);
    let rules = tcl_syntax::word_rules::WordValueRules::from_config(&config);
    words
        .into_iter()
        .zip(tokens.words())
        .map(|(mut word, original)| {
            let value = crate::registry_invocation::effective_invocation_word(
                original,
                config.escapes,
                rules,
            );
            if word.source_available
                && let Some(value) = value.as_registry_word().literal()
            {
                value.clone_into(&mut word.text);
                word.substituted = false;
            }
            word
        })
        .collect()
}

fn value_commands(text: &str, config: LexerConfig) -> Vec<(bool, Vec<CommandWord>)> {
    let source = SourceMap::new(text);
    crate::segmenter::segment_commands_with_offset_and_config(
        text,
        0,
        tcl_lexer::SourceSegmentationPurpose::NestedScript.config(config),
    )
    .iter()
    .map(|command| {
        (
            command.is_partial,
            footprint_command_words(&source, config, command),
        )
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
    if context.reads == ReadPurpose::InterpolationAndNames {
        let mut scanner = crate::var_refs::VarReferenceScanner::with_config(
            crate::var_refs::VarScanOptions::default(),
            context.config,
        );
        out.read_names
            .extend(scanner.scan_lexical_script(text, context.registry));
    }
    for (partial, words) in value_commands(text, context.config) {
        out.opaque |= partial;
        for word in words.iter().filter(|word| !word.braced_literal) {
            if !word.source_available {
                out.opaque = true;
                continue;
            }
            let lexer = tcl_lexer::Lexer::with_config(
                &word.raw,
                tcl_lexer::SourceSegmentationPurpose::NestedScript.config(context.config),
            );
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
    if let Some(purpose) = context.ownership {
        out.include_possible(&selected_name_ownership(
            words,
            context,
            state,
            holder.as_ref(),
            purpose,
        ));
    } else {
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
    }
    reevaluated_reads(words, context, state, holder.as_ref(), out, depth);
    state.source_order_registry_barrier_for_command_with_metadata_context(
        words,
        false,
        context.registry,
        context.namespace,
        Traits::EVALUATES_CODE,
        Some(context.metadata),
    );
}

/// Conditional named operands under the exact materialized command horizon.
/// The Registry owns role grammar, local alias names and list/root semantics.
fn selected_name_ownership(
    words: &[CommandWord],
    context: &FootprintContext<'_>,
    state: &ModuleCommandBindings,
    holder: &crate::command_binding::SourceNamespaceKey,
    purpose: crate::script_binds::Ownership,
) -> VariableWriteEffects {
    use tcl_registry::source_name_ownership::{SourceNameOwnershipPurpose, SourceRolePurpose};
    let Some(source_purpose) =
        state.retained_original_source_purpose(context.metadata, context.registry)
    else {
        return VariableWriteEffects {
            opaque: true,
            ..Default::default()
        };
    };
    let role_purpose =
        if source_purpose == crate::command_binding::OriginalInvocationSourcePurpose::Logical {
            SourceRolePurpose::Logical
        } else {
            SourceRolePurpose::Original
        };
    let purpose = match purpose {
        crate::script_binds::Ownership::Bindings => SourceNameOwnershipPurpose::Bindings,
        #[cfg(test)]
        crate::script_binds::Ownership::DecodedBindings => SourceNameOwnershipPurpose::Bindings,
        crate::script_binds::Ownership::BindingsOrNameReads => {
            SourceNameOwnershipPurpose::BindingsOrNameReads
        }
        crate::script_binds::Ownership::ScopeAliases => SourceNameOwnershipPurpose::ScopeAliases,
        crate::script_binds::Ownership::ScopeAliasesAndTraceRegistrations => {
            SourceNameOwnershipPurpose::ScopeAliasesAndTraceRegistrations
        }
    };
    let mut found = false;
    let mut out = VariableWriteEffects::default();
    state.for_each_resolved_command_words(words, holder, |target, invocation| {
        found = true;
        if !target.registry_backed {
            out.opaque = true;
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
            out.opaque = true;
            return;
        };
        let ownership = schema.authored_source_name_ownership(role_purpose, purpose);
        out.include_possible(&VariableWriteEffects {
            names: ownership.bindings,
            read_names: ownership.by_name_reads,
            opaque: ownership.opaque,
        });
    });
    out.opaque |= !found;
    out
}

fn reevaluated_reads(
    words: &[CommandWord],
    context: &FootprintContext<'_>,
    state: &mut ModuleCommandBindings,
    holder: &crate::command_binding::SourceNamespaceKey,
    out: &mut VariableWriteEffects,
    depth: u32,
) {
    let (bodies, expressions, templates, opaque) = immediate_values(words, context, state, holder);
    out.opaque |= opaque;
    for plan in templates {
        if plan.dynamic && (plan.kinds.commands || plan.kinds.variables) {
            out.opaque = true;
        }
        out.read_names
            .extend(plan.reads.into_iter().map(|read| read.name));
        for region in plan.script_regions {
            let mut branch = state.clone();
            script_writes(&region.script.script, context, &mut branch, out, depth + 1);
            state.join_possible_source_effects(&branch);
        }
    }
    let mut scanner = crate::var_refs::VarReferenceScanner::with_config(
        crate::var_refs::VarScanOptions::default(),
        context.config,
    );
    for expression in expressions {
        if context.reads == ReadPurpose::InterpolationAndNames {
            out.read_names
                .extend(scanner.scan_lexical_word(&expression, context.registry));
        }
        let map = SourceMap::new(&expression);
        for token in tcl_lexer::Lexer::with_config(
            &expression,
            tcl_lexer::SourceSegmentationPurpose::NestedScript.config(context.config),
        )
        .as_quoted_body()
        {
            let Ok(token) = token else {
                out.opaque = true;
                continue;
            };
            if token.kind == tcl_lexer::TokenType::Cmd {
                let mut branch = state.clone();
                script_writes(map.token_text(token), context, &mut branch, out, depth + 1);
                state.join_possible_source_effects(&branch);
            }
        }
    }
    for body in bodies {
        let mut branch = state.clone();
        script_writes(&body, context, &mut branch, out, depth + 1);
        state.join_possible_source_effects(&branch);
    }
}

fn immediate_values(
    words: &[CommandWord],
    context: &FootprintContext<'_>,
    state: &ModuleCommandBindings,
    holder: &crate::command_binding::SourceNamespaceKey,
) -> (
    Vec<String>,
    Vec<String>,
    Vec<tcl_registry::value_transfer::TemplateWordPlan>,
    bool,
) {
    let Some(source_purpose) =
        state.retained_original_source_purpose(context.metadata, context.registry)
    else {
        return (Vec::new(), Vec::new(), Vec::new(), true);
    };
    let mut bodies = Vec::new();
    let mut expressions = Vec::new();
    let mut templates = Vec::new();
    let mut opaque = false;
    state.for_each_resolved_command_words(words, holder, |target, invocation| {
        if !target.registry_backed {
            opaque = true;
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
            opaque = true;
            return;
        };
        bodies.extend(immediate_same_frame_script_values_for_role_purpose(
            &schema,
            context.ownership.is_some()
                && (source_purpose
                    == crate::command_binding::OriginalInvocationSourcePurpose::Logical),
        ));
        if schema
            .semantics
            .traits
            .contains(Traits::PERFORMS_SUBSTITUTION)
        {
            match crate::value_transfer::source_template_plan(
                &schema,
                context.registry,
                context.metadata,
                context.config,
            ) {
                Some(plan) => templates.push(plan),
                None => opaque = true,
            }
        }
        let logical = context.ownership.is_some()
            && (source_purpose == crate::command_binding::OriginalInvocationSourcePurpose::Logical);
        if context.ownership.is_some() {
            opaque |= immediate_body_ownership_is_opaque(&schema, logical);
        }
        let layout = if logical {
            schema.authored_logical_source_expression_arguments()
        } else {
            schema.authored_source_expression_arguments()
        };
        let Some(layout) = layout else {
            opaque = true;
            return;
        };
        let arguments = schema.words.arguments();
        if layout.concatenates {
            if let Some(values) = layout
                .arguments
                .iter()
                .map(|ordinal| arguments.literal_at(*ordinal))
                .collect::<Option<Vec<_>>>()
            {
                expressions.push(values.join(" "));
            } else {
                opaque = true;
            }
        } else {
            for ordinal in layout.arguments {
                if let Some(value) = arguments.literal_at(ordinal) {
                    expressions.push(value.to_owned());
                } else {
                    opaque = true;
                }
            }
        }
    });
    (bodies, expressions, templates, opaque)
}

fn immediate_body_ownership_is_opaque(
    schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    logical: bool,
) -> bool {
    if schema.semantics.body_kind != BodyKind::Plain
        || schema
            .semantics
            .body_interpreter
            .resolve(schema.words.arguments())
            != tcl_registry::InterpreterScope::Current
    {
        return false;
    }
    let arguments = if logical {
        schema.authored_logical_source_plain_script_arguments()
    } else {
        schema.authored_source_plain_script_arguments()
    };
    let Some(arguments) = arguments else {
        return true;
    };
    arguments.into_iter().any(|ordinal| {
        schema.authored_source_script_timing_at(ordinal) == Some(ScriptTiming::SameInvocation)
            && (schema.words.arguments().literal_at(ordinal).is_none()
                || (schema
                    .semantics
                    .traits
                    .contains(Traits::SCRIPT_CONCATENATES_ARGS)
                    && (ordinal..schema.words.arguments().len())
                        .any(|index| schema.words.arguments().literal_at(index).is_none())))
    })
}

/// Static values in immediate current-frame bodies of an already selected
/// descriptor. This is possible source evaluation, never entered-script proof.
pub(crate) fn immediate_same_frame_script_values(
    schema: &tcl_registry::ResolvedInvocation<'_, '_>,
) -> Vec<String> {
    immediate_same_frame_script_values_for_role_purpose(schema, false)
}

fn immediate_same_frame_script_values_for_role_purpose(
    schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    logical: bool,
) -> Vec<String> {
    if schema.semantics.body_kind != BodyKind::Plain
        || schema
            .semantics
            .body_interpreter
            .resolve(schema.words.arguments())
            != tcl_registry::InterpreterScope::Current
    {
        return Vec::new();
    }
    let (roles, complete) = if logical {
        schema.authored_logical_source_argument_roles()
    } else {
        schema.authored_source_argument_roles()
    };
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
                None if logical => tcl_syntax::list::concat_values(values),
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

#[cfg(test)]
mod original_materialized_word_purpose_tests {
    use super::*;

    #[test]
    fn original_materialized_body_keeps_bom_value_and_parent_word_rules_separate() {
        // naming.source.original-configured-script-completeness
        // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
        // Original body geometry and materialized value syntax, without child entry or complete effect closure.
        let config = LexerConfig {
            leading_bom: tcl_lexer::LeadingBom::Skip,
            braced_var: tcl_lexer::BracedVarStyle::FirstClose,
            strict_quoting: true,
            ..LexerConfig::default()
        };
        let source = "\u{feff}eval {\u{feff}#prefix $x}";
        let image = tcl_lexer::SourceImage::document(source);
        let plan = tcl_lexer::native_script_words_in(
            image.clone(),
            tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
            config,
        )
        .unwrap();
        let word = &plan.commands[0].words[1];
        assert_eq!(word.config(), config);
        assert_eq!(word.image(), &image);
        let body = source.get(word.content_span().unwrap().as_range()).unwrap();
        let commands = value_commands(body, config);
        assert_eq!(commands.len(), 1);
        assert!(!commands[0].0);
        assert_eq!(commands[0].1[0].literal(), Some("\u{feff}#prefix"));
        assert_eq!(commands[0].1[1].raw, "$x");
        assert!(commands[0].1[1].substituted);
        assert_eq!(
            commands[0].1[0].raw,
            value_commands(body, config.nested())[0].1[0].raw
        );
        assert!(value_commands("#prefix $x", config).is_empty());
        let first_close = value_commands("\u{feff}#prefix ${a{b}", config);
        assert_eq!(first_close[0].1[1].raw, "${a{b}");
        assert_eq!(config.leading_bom, tcl_lexer::LeadingBom::Skip);
    }
}

#[cfg(test)]
mod original_written_footprint_tests {
    use super::*;

    #[test]
    fn original_materialized_written_words_do_not_reborrow_component_spans() {
        // naming.source.original-configured-script-completeness
        // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
        let config = LexerConfig {
            braced_var: tcl_lexer::BracedVarStyle::FirstClose,
            ..LexerConfig::default()
        };
        let source = "list ${a{b} \"[set é]$x\" {é $x}";
        let map = SourceMap::new(source);
        let command =
            crate::segmenter::segment_commands_with_offset_and_config(source, 0, config.nested())
                .remove(0);
        let words = footprint_command_words(&map, config, &command);
        assert_eq!(words[1].raw, "${a{b}");
        assert_eq!(words[2].raw, "\"[set é]$x\"");
        assert_eq!(words[3].raw, "{é $x}");
        assert_eq!(words[3].literal(), Some("é $x"));
        assert!(words[1].substituted);
        let mut changed = command.clone();
        changed.all_tokens.clear();
        assert!(
            footprint_command_words(&map, config, &changed)
                .iter()
                .all(|word| word.literal().is_none())
        );
    }
}
