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

//! Method-name references captured from the operands of a class definition.

use super::super::{
    Arc, InvocationFacts, ModuleCommandBindings, SourceExecutionContext, SourceNativeInvocation,
    retained_script_operand, source_effective_words,
};
use super::closed_members::{class_definition_block, original_definition_member_target};
use super::retained_entries::{
    apply_declared_visibility, original_member_input, retained_method_entry,
};
use super::{
    DefinitionReferenceScope, MethodParameterSource, SourceMethodReceiver,
    SourceReceiverMethodEntries, SourceReceiverMethodEntry,
};
use tcl_core_types::NameBytes;
use tcl_names::command::CommandAllocationSite;
use tcl_names::origin::ExecutedScriptSource;

pub(super) fn standalone_definition_class<'a>(
    native: SourceNativeInvocation<'_>,
    facts: &InvocationFacts,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'a>,
) -> Option<(
    super::super::SourceCommandTarget,
    &'a tcl_registry::definer::DefinitionBodyGrammar,
)> {
    let mut configured = facts
        .state_transitions
        .declared()?
        .facts()
        .iter()
        .filter_map(|fact| match &fact.transition {
            tcl_registry::StateTransition::ObjectDispatch(
                tcl_registry::ObjectDispatchTransition::Configure {
                    target,
                    layer: tcl_registry::ObjectDispatchLayer::Class,
                },
            ) => target.argument_index(),
            _ => None,
        });
    let name = configured.next()?;
    if !native.target.registry_backed
        || !native.target.prepended.is_empty()
        || native.target.implementation_generation != 0
        || state.source_step_observed()
        || state.source_execution_observed(native.target.identity.as_ref())
    {
        return None;
    }
    let grammar = context
        .registry
        .get_for_surface(
            &native.target.command,
            state
                .baseline
                .dialect?
                .authoring_query()
                .map(|query| query.with_realm(context.realm())),
        )?
        .definition_body?;
    if configured.next().is_some() {
        return None;
    }
    let input = native
        .original_variable_operands
        .input(name, &state.source_variables)?;
    let binding = super::super::source_binding_from_original_input(
        state,
        input,
        &context.namespace_identity(),
    )?;
    let class = binding.proved_target()?;
    let identity = class.identity.as_ref()?;
    for path in state.original_command_paths_for_input(&context.namespace_identity(), input)? {
        let mut occupied = false;
        for slot in path {
            let raw = state.original_bindings_for_key(&slot)?;
            if raw
                .iter()
                .all(|binding| matches!(binding, super::super::MayBinding::Missing))
            {
                continue;
            }
            if raw.len() != 1
                || !matches!(raw.iter().next(), Some(super::super::MayBinding::Target(target))
                if target.kind == tcl_names::command::BindingKind::Class && target.terminal
                    && target.prepended.is_empty() && target.token.as_ref() == Some(identity))
            {
                return None;
            }
            occupied = true;
            break;
        }
        if !occupied {
            return None;
        }
    }
    let definition = state.class_definitions.get(identity)?;
    (definition.dispatcher.is_none()
        && state.class_definition_dependencies_hold(definition)
        && !state.tainted_object_dispatch.contains("*")
        && !state.tainted_object_dispatch.contains(&class.command))
    .then(|| (class.clone(), grammar))
}

pub(super) fn capture_standalone_definition_references(
    native: SourceNativeInvocation<'_>,
    facts: &InvocationFacts,
    grammar: &tcl_registry::definer::DefinitionBodyGrammar,
    class: &super::super::SourceCommandTarget,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> Option<Vec<super::super::definition_method_references::SourceDefinitionMethodReferencePhase>> {
    super::super::native_result::numeric_call_arguments(context, native.target)?;
    context.written_representations?;
    let entries = &state
        .class_definitions
        .get(class.identity.as_ref()?)?
        .receiver_method_entries;
    let arguments = native.invocation.arguments();
    if arguments.len() == 2 && facts.arg_roles.contains(&(1, tcl_registry::ArgRole::Body)) {
        let definition = retained_script_operand(
            1,
            native.script_operands(),
            state,
            native.segment.span.start(),
            context.config,
        )?;
        let map =
            tcl_lexer::SourceMap::from_image(&definition.text).with_base(definition.base(), 0, 0);
        let segments = tcl_source::segment::segment_commands_image_with_offset_and_config(
            &definition.text,
            definition.base(),
            context.config,
        )?;
        let mut references = Vec::new();
        for segment in segments {
            let words = crate::ir::CommandTokens::from_segmented(&map, context.config, &segment);
            let values = source_effective_words(words.words(), state.baseline.dialect, None);
            let member = grammar.member(values.first()?.as_registry_word().literal()?)?;
            // Only visibility rows preserve this at-phase method inventory.
            // Retraction, slot options and arbitrary definition bodies need
            // independent effects before later operands can use that table.
            member.visibility_effect?;
            let worker = original_definition_member_target(
                grammar,
                member,
                tcl_registry::definer::DefinitionReceiver::Instance,
                state,
                context,
            )?;
            references.extend(capture_reference_operands(
                &DefinitionReferenceInvocation {
                    receiver: SourceMethodReceiver::Instance,
                    worker,
                    site: CommandAllocationSite {
                        source: Arc::clone(&definition.origin),
                        offset: segment.span.start(),
                    },
                    config: context.config,
                    policy: state.baseline.execution_name_policy?.native_recipe()?,
                },
                &words,
                &values,
                entries,
            )?);
        }
        return Some(references);
    }
    let member = grammar.member(arguments.literal_at(1)?)?;
    member.visibility_effect?;
    let worker = original_definition_member_target(
        grammar,
        member,
        tcl_registry::definer::DefinitionReceiver::Instance,
        state,
        context,
    )?;
    let source = state.current_source_origin.as_ref()?;
    native
        .words
        .iter()
        .skip(3)
        .enumerate()
        .map(|(index, operand)| {
            let name_input = native
                .original_variable_operands
                .input(index + 2, &state.source_variables)?
                .clone();
            let name = std::str::from_utf8(name_input.bytes())
                .unwrap_or_default()
                .to_owned();
            let entry = SourceReceiverMethodEntry::for_original_input(
                entries,
                SourceMethodReceiver::Instance,
                &name_input,
            )
            .cloned();
            super::super::definition_method_references::SourceDefinitionMethodReferencePhase::capture(
                super::super::definition_method_references::DefinitionMethodReferenceCapture {
                    receiver: SourceMethodReceiver::Instance,
                    worker: worker.clone(),
                    invocation: CommandAllocationSite {
                        source: Arc::clone(source),
                        offset: native.segment.span.start(),
                    },
                    operand: operand.clone(),
                    name,
                    name_input,
                    entry,
                    config: context.config,
                },
            )
        })
        .collect()
}

pub(super) fn capture_definition_method_references(
    grammar: &tcl_registry::definer::DefinitionBodyGrammar,
    definition: &ExecutedScriptSource,
    map: &tcl_lexer::SourceMap<'_>,
    segments: &[tcl_source::segment::SegmentedCommand],
    state: &ModuleCommandBindings,
    scope: DefinitionReferenceScope,
    context: SourceExecutionContext<'_>,
) -> Option<Vec<super::super::definition_method_references::SourceDefinitionMethodReferencePhase>> {
    let mut entries = SourceReceiverMethodEntries::new();
    let mut references = Vec::new();
    for segment in segments {
        if let Some(block) = class_definition_block(
            grammar,
            definition,
            map,
            segment,
            state.baseline.dialect,
            context,
        ) {
            references.extend(capture_class_block_references(
                grammar,
                &block,
                state,
                scope.parameters,
                context,
            )?);
            continue;
        }
        let words = crate::ir::CommandTokens::from_segmented(map, context.config, segment);
        let values = source_effective_words(words.words(), state.baseline.dialect, None);
        let head = values.first()?.as_registry_word().literal()?;
        let member = grammar.member(head)?;
        retain_inherited_reference_entries(grammar, member, &values, state, context, &mut entries);
        if member.all_args_ref == Some(tcl_registry::definer::MemberRefKind::Method) {
            let receiver = match scope.receiver {
                SourceMethodReceiver::Instance => {
                    tcl_registry::definer::DefinitionReceiver::Instance
                }
                SourceMethodReceiver::Class => tcl_registry::definer::DefinitionReceiver::Class,
            };
            let worker =
                original_definition_member_target(grammar, member, receiver, state, context)?;
            references.extend(capture_reference_operands(
                &DefinitionReferenceInvocation {
                    receiver: scope.receiver,
                    worker,
                    site: CommandAllocationSite {
                        source: Arc::clone(&definition.origin),
                        offset: segment.span.start(),
                    },
                    config: context.config,
                    policy: state.baseline.execution_name_policy?.native_recipe()?,
                },
                &words,
                &values,
                &entries,
            )?);
        }
        if let Some(name) = original_forward_member_name(
            grammar,
            definition,
            &words,
            segment.span.start(),
            state,
            context,
        ) {
            entries.remove(&(scope.receiver, name));
        }
        apply_declared_visibility(
            grammar,
            map,
            segment,
            state.baseline.dialect,
            context,
            &mut entries,
        );
        if let Some(mut entry) = retained_method_entry(
            grammar,
            definition,
            map,
            segment,
            state.baseline.dialect,
            MethodParameterSource {
                parameters: scope.parameters,
                policy: state.baseline.execution_name_policy?.native_recipe()?,
            },
            context,
        ) {
            if scope.receiver == SourceMethodReceiver::Class {
                entry.receiver = SourceMethodReceiver::Class;
            }
            references.push(capture_method_declaration_reference(
                grammar, &words, &values, state, &entry, context,
            )?);
            entries.insert(
                (entry.receiver, NameBytes::from(entry.original_name.bytes())),
                entry,
            );
        }
    }
    Some(references)
}

fn retain_inherited_reference_entries(
    grammar: &tcl_registry::definer::DefinitionBodyGrammar,
    member: &tcl_registry::definer::MemberSpec,
    values: &[crate::registry_invocation::EffectiveInvocationWord],
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
    entries: &mut SourceReceiverMethodEntries,
) {
    if let Some((_, inherited)) = state.closed_inherited_member(grammar, member, values, context) {
        for (key, entry) in inherited.receiver_method_entries.iter() {
            if key.0 == SourceMethodReceiver::Instance {
                entries.entry(key.clone()).or_insert_with(|| entry.clone());
            }
        }
    }
}

fn original_forward_member_name(
    grammar: &tcl_registry::definer::DefinitionBodyGrammar,
    definition: &ExecutedScriptSource,
    words: &crate::ir::CommandTokens,
    offset: u32,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> Option<NameBytes> {
    let dialect = state.baseline.dialect?;
    let policy = state.baseline.execution_name_policy?.native_recipe()?;
    let forward = super::super::deferred_forward::original_forward_method(
        grammar, definition, words, offset, dialect, policy, context,
    )?;
    Some(NameBytes::from(forward.name_input().bytes()))
}

fn capture_method_declaration_reference(
    grammar: &tcl_registry::definer::DefinitionBodyGrammar,
    words: &crate::ir::CommandTokens,
    values: &[crate::registry_invocation::EffectiveInvocationWord],
    state: &ModuleCommandBindings,
    entry: &SourceReceiverMethodEntry,
    context: SourceExecutionContext<'_>,
) -> Option<super::super::definition_method_references::SourceDefinitionMethodReferencePhase> {
    use super::super::definition_method_references::{
        DefinitionMethodReferenceCapture, SourceDefinitionMethodReferencePhase,
    };
    let strings = values
        .iter()
        .map(crate::registry_invocation::EffectiveInvocationWord::literal_bytes)
        .collect::<Option<Vec<_>>>()?;
    let layout = grammar.receiver_method_layout_bytes(
        strings.first()?,
        &strings[1..],
        state
            .baseline
            .dialect?
            .authoring_query()
            .map(|query| query.with_realm(context.realm())),
    )?;
    let name_at = layout
        .member
        .arg_roles
        .iter()
        .find_map(|(index, role)| {
            (*role == tcl_registry::ArgRole::Name).then_some(usize::from(*index))
        })?
        .checked_add(layout.member_word)?
        .checked_add(1)?;
    let worker_receiver = if entry.receiver == SourceMethodReceiver::Class && !layout.class_delegate
    {
        tcl_registry::definer::DefinitionReceiver::Class
    } else {
        layout.worker_receiver
    };
    let worker =
        original_definition_member_target(grammar, layout.member, worker_receiver, state, context)?;
    SourceDefinitionMethodReferencePhase::capture(DefinitionMethodReferenceCapture {
        receiver: entry.receiver,
        worker,
        invocation: entry.declaration().clone(),
        operand: words.words().get(name_at)?.clone(),
        name: entry.name.clone(),
        name_input: entry.original_name.clone(),
        entry: Some(entry.clone()),
        config: context.config,
    })
}

fn capture_class_block_references(
    grammar: &tcl_registry::definer::DefinitionBodyGrammar,
    block: &ExecutedScriptSource,
    state: &ModuleCommandBindings,
    parameters: tcl_dialect::ParameterGrammar,
    context: SourceExecutionContext<'_>,
) -> Option<Vec<super::super::definition_method_references::SourceDefinitionMethodReferencePhase>> {
    let map = tcl_lexer::SourceMap::from_image(&block.text).with_base(block.base(), 0, 0);
    let segments = tcl_source::segment::segment_commands_image_with_offset_and_config(
        &block.text,
        block.base(),
        context.config,
    )?;
    capture_definition_method_references(
        grammar,
        block,
        &map,
        &segments,
        state,
        DefinitionReferenceScope {
            parameters,
            receiver: SourceMethodReceiver::Class,
        },
        context,
    )
}

struct DefinitionReferenceInvocation {
    pub(super) receiver: SourceMethodReceiver,
    pub(super) worker: super::super::SourceCommandTarget,
    site: CommandAllocationSite,
    pub(super) config: tcl_lexer::LexerConfig,
    policy: tcl_syntax::naming::NamePolicyProtocol,
}

fn capture_reference_operands(
    phase: &DefinitionReferenceInvocation,
    words: &crate::ir::CommandTokens,
    values: &[crate::registry_invocation::EffectiveInvocationWord],
    entries: &SourceReceiverMethodEntries,
) -> Option<Vec<super::super::definition_method_references::SourceDefinitionMethodReferencePhase>> {
    use super::super::definition_method_references::{
        DefinitionMethodReferenceCapture, SourceDefinitionMethodReferencePhase,
    };
    words
        .words()
        .iter()
        .zip(values)
        .skip(1)
        .enumerate()
        .map(|(index, (operand, value))| {
            value.literal_bytes()?;
            let name_input = original_member_input(
                &phase.site.source,
                words,
                phase.site.offset,
                index + 1,
                phase.policy,
                phase.config,
            )?;
            let bytes = name_input.bytes();
            SourceDefinitionMethodReferencePhase::capture(DefinitionMethodReferenceCapture {
                receiver: phase.receiver,
                worker: phase.worker.clone(),
                invocation: phase.site.clone(),
                operand: operand.clone(),
                name: std::str::from_utf8(bytes).unwrap_or_default().to_owned(),
                entry: entries
                    .get(&(phase.receiver, NameBytes::from(bytes)))
                    .cloned(),
                name_input,
                config: phase.config,
            })
        })
        .collect()
}
