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

//! What a bounded class definition retains: methods, lifecycle bodies and instance variables.

use super::super::{
    Arc, DeferredSourceBody, InvocationFacts, ModuleCommandBindings, SourceExecutionContext,
    SourceNativeInvocation, retained_script_operand, source_effective_words,
};
use super::closed_members::{class_definition_block, closed_definition_member};
use super::{
    BoundedMethodDefinition, MethodParameterSource, RetainedClassEntries, SourceMethodReceiver,
    SourceReceiverMethodEntries, SourceReceiverMethodEntry,
};
use crate::command_binding::ExecutedScriptWords;
use crate::signature_scan::scope::SignatureSourceNameInput;
use tcl_core_types::NameBytes;
use tcl_names::command::CommandAllocationSite;
use tcl_names::origin::ExecutedScriptSource;
use tcl_names::word::SignatureSourceNameKey;

pub(super) fn closed_bounded_method_segments(
    grammar: &tcl_registry::definer::DefinitionBodyGrammar,
    definition: &ExecutedScriptSource,
    map: &tcl_lexer::SourceMap<'_>,
    segments: &[tcl_source::segment::SegmentedCommand],
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> bool {
    segments.iter().all(|segment| {
        let closed = closed_definition_member(grammar, definition, map, segment, state, context);
        #[cfg(test)]
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
            eprintln!(
                "ORIGINAL_OO_MEMBER offset={} keyword={} closed={closed}",
                segment.span.start(),
                segment.name()
            );
        }
        closed
    })
}

pub(super) fn select_bounded_method_definition(
    native: SourceNativeInvocation<'_>,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
    facts: &InvocationFacts,
) -> Option<BoundedMethodDefinition> {
    #[cfg(test)]
    if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
        eprintln!(
            "ORIGINAL_OO_REGISTRATION offset={} selected={} kind={:?} opaque={} policy={:?} roles={:?}",
            native.segment.span.start(),
            native.target.command,
            native.target.kind,
            state.has_opaque_domain(),
            state.baseline.execution_name_policy,
            facts.arg_roles
        );
    }
    let grammar = context
        .registry
        .get_for_surface(
            &native.target.command,
            state
                .baseline
                .dialect
                .and_then(tcl_native_model::InvocationDialect::authoring_query)
                .map(|query| query.with_realm(context.realm())),
        )
        .and_then(|spec| spec.definition_body)?;
    let original_factory =
        super::super::registered_class_factory::OriginalClassFactoryState::capture(
            native, state, context,
        )?;
    if !state.default_construction_dependencies_hold(grammar) {
        #[cfg(test)]
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
            eprintln!(
                "ORIGINAL_OO_REGISTRATION declined=factory-dependencies offset={}",
                native.segment.span.start()
            );
        }
        return None;
    }
    let argument = facts.arg_roles.iter().find_map(|(index, role)| {
        (*role == tcl_registry::ArgRole::Body)
            .then_some(facts.argument_offset + usize::from(*index))
    })?;
    let definition = retained_script_operand(
        argument,
        native.script_operands(),
        state,
        native.segment.span.start(),
        context.config,
    )?;
    let parameters = state
        .baseline
        .dialect
        .and_then(tcl_native_model::InvocationDialect::parameter_grammar)?;
    Some(BoundedMethodDefinition {
        grammar,
        original_factory,
        definition,
        parameters,
    })
}

pub(super) fn retained_class_entries(
    grammar: &tcl_registry::definer::DefinitionBodyGrammar,
    definition: &ExecutedScriptSource,
    map: &tcl_lexer::SourceMap<'_>,
    segments: &[tcl_source::segment::SegmentedCommand],
    dialect: Option<tcl_native_model::InvocationDialect>,
    source: MethodParameterSource,
    context: SourceExecutionContext<'_>,
) -> RetainedClassEntries {
    let MethodParameterSource { parameters, policy } = source;
    let lifecycle = retained_lifecycle_entries(
        grammar, definition, map, segments, dialect, parameters, context,
    );
    let lifecycle_entries_closed = lifecycle.is_some();
    let lifecycle = lifecycle.unwrap_or_default();
    let forward_method_entries = super::super::deferred_forward::retained_forward_methods(
        grammar, definition, map, segments, dialect, source, context,
    );
    let mut receiver_method_entries =
        retained_method_entries(grammar, definition, map, segments, dialect, source, context);
    Arc::make_mut(&mut receiver_method_entries).retain(|(receiver, name), _| {
        *receiver != SourceMethodReceiver::Instance || !forward_method_entries.contains_key(name)
    });
    RetainedClassEntries {
        inherited_classes: Arc::default(),
        single_superclass: false,
        instance_methods: closed_instance_methods(
            grammar, definition, map, segments, dialect, policy, context,
        ),
        instance_variables: retained_instance_variables(
            grammar, definition, map, segments, dialect, policy, context,
        ),
        constructor_entry: lifecycle.constructor,
        constructor_provider: None,
        destructor_entry: lifecycle.destructor,
        destructor_provider: None,
        lifecycle_entries_closed,
        receiver_method_entries,
        forward_method_entries,
    }
}

// These names come from the admitted instance-side definition members, not
// from method source or the reported class command spelling.
fn retained_instance_variables(
    grammar: &tcl_registry::definer::DefinitionBodyGrammar,
    definition: &ExecutedScriptSource,
    map: &tcl_lexer::SourceMap<'_>,
    segments: &[tcl_source::segment::SegmentedCommand],
    dialect: Option<tcl_native_model::InvocationDialect>,
    policy: tcl_syntax::naming::NamePolicyProtocol,
    context: SourceExecutionContext<'_>,
) -> Option<Arc<super::super::receiver_variable_inventory::OriginalReceiverVariableInventory>> {
    let mut inventory =
        super::super::receiver_variable_inventory::OriginalReceiverVariableInventory::empty(
            grammar, policy,
        )?;
    for segment in segments {
        let words = crate::ir::CommandTokens::from_segmented(map, context.config, segment);
        let values = source_effective_words(words.words(), dialect, None);
        let member = grammar.member(values.first()?.as_registry_word().literal()?)?;
        if !member.all_args_var {
            continue;
        }
        // Expanded arguments need their complete original sibling mapping;
        // the written vector alone cannot certify a declaration inventory.
        if words
            .words()
            .iter()
            .any(|word| matches!(word, tcl_source::words::WordExpr::Expand { .. }))
        {
            return None;
        }
        let arguments = (1..words.words().len())
            .map(|ordinal| {
                original_member_input(
                    &definition.origin,
                    &words,
                    segment.span.start(),
                    ordinal,
                    policy,
                    context.config,
                )
            })
            .collect::<Option<Vec<_>>>()?;
        inventory.apply(member.slot, arguments)?;
    }
    Some(Arc::new(inventory))
}

fn retained_method_entries(
    grammar: &tcl_registry::definer::DefinitionBodyGrammar,
    definition: &ExecutedScriptSource,
    map: &tcl_lexer::SourceMap<'_>,
    segments: &[tcl_source::segment::SegmentedCommand],
    dialect: Option<tcl_native_model::InvocationDialect>,
    source: MethodParameterSource,
    context: SourceExecutionContext<'_>,
) -> Arc<SourceReceiverMethodEntries> {
    let mut entries = super::super::BTreeMap::new();
    for segment in segments {
        if let Some(block) =
            class_definition_block(grammar, definition, map, segment, dialect, context)
        {
            let block_map =
                tcl_lexer::SourceMap::from_image(&block.text).with_base(block.base(), 0, 0);
            let Some(inner) = tcl_source::segment::segment_commands_image_with_offset_and_config(
                &block.text,
                block.base(),
                context.config,
            ) else {
                continue;
            };
            for ((_, name), mut entry) in retained_method_entries(
                grammar, &block, &block_map, &inner, dialect, source, context,
            )
            .as_ref()
            .clone()
            {
                entry.receiver = SourceMethodReceiver::Class;
                entries.insert((SourceMethodReceiver::Class, name), entry);
            }
            continue;
        }
        apply_declared_visibility(grammar, map, segment, dialect, context, &mut entries);
        if let Some(entry) =
            retained_method_entry(grammar, definition, map, segment, dialect, source, context)
        {
            entries.insert(
                (entry.receiver, NameBytes::from(entry.original_name.bytes())),
                entry,
            );
        }
    }
    Arc::new(entries)
}

// Visibility changes only a method already installed at this definition
// phase. A later method declaration starts from its native default again.
pub(super) fn apply_declared_visibility(
    grammar: &tcl_registry::definer::DefinitionBodyGrammar,
    map: &tcl_lexer::SourceMap<'_>,
    segment: &tcl_source::segment::SegmentedCommand,
    dialect: Option<tcl_native_model::InvocationDialect>,
    context: SourceExecutionContext<'_>,
    entries: &mut SourceReceiverMethodEntries,
) {
    let words = crate::ir::CommandTokens::from_segmented(map, context.config, segment);
    let values = source_effective_words(words.words(), dialect, None);
    let Some(effect) = values
        .first()
        .and_then(|word| word.as_registry_word().literal())
        .and_then(|head| grammar.member(head))
        .and_then(|member| member.visibility_effect)
    else {
        return;
    };
    for name in values
        .iter()
        .skip(1)
        .filter_map(crate::registry_invocation::EffectiveInvocationWord::literal_bytes)
    {
        if let Some(entry) =
            entries.get_mut(&(SourceMethodReceiver::Instance, NameBytes::from(name)))
        {
            entry.exported = effect == tcl_registry::definer::MemberVisibility::Exported;
        }
    }
}

pub(in super::super) fn original_member_input(
    origin: &tcl_names::origin::SourceOriginId,
    words: &crate::ir::CommandTokens,
    offset: u32,
    ordinal: usize,
    policy: tcl_syntax::naming::NamePolicyProtocol,
    config: tcl_lexer::LexerConfig,
) -> Option<SignatureSourceNameInput> {
    let original = crate::registry_invocation::original_native_compiler_words(
        origin.source_image(),
        words.words(),
        offset,
        config,
    )?;
    Some(SignatureSourceNameInput::OriginalWord(
        SignatureSourceNameKey::from_original_native_word(
            original.get(ordinal)?,
            tcl_syntax::word_rules::WordValueRules::from_config(&config),
            policy,
        )?,
    ))
}

pub(in super::super) fn retained_method_entry(
    grammar: &tcl_registry::definer::DefinitionBodyGrammar,
    definition: &ExecutedScriptSource,
    map: &tcl_lexer::SourceMap<'_>,
    segment: &tcl_source::segment::SegmentedCommand,
    dialect: Option<tcl_native_model::InvocationDialect>,
    source: MethodParameterSource,
    context: SourceExecutionContext<'_>,
) -> Option<SourceReceiverMethodEntry> {
    let MethodParameterSource { parameters, policy } = source;
    let words = crate::ir::CommandTokens::from_segmented(map, context.config, segment);
    let values = source_effective_words(words.words(), dialect, None);
    let head = values.first()?.as_registry_word().literal()?;
    let arguments = values
        .iter()
        .skip(1)
        .map(crate::registry_invocation::EffectiveInvocationWord::literal_bytes)
        .collect::<Option<Vec<_>>>()?;
    let query = dialect
        .and_then(tcl_native_model::InvocationDialect::authoring_query)
        .map(|query| query.with_realm(context.realm()));
    let layout = grammar.receiver_method_layout_bytes(head.as_bytes(), &arguments, query)?;
    let receiver = match layout.receiver {
        tcl_registry::definer::DefinitionReceiver::Instance => SourceMethodReceiver::Instance,
        tcl_registry::definer::DefinitionReceiver::Class => SourceMethodReceiver::Class,
    };
    let name_at = layout.member.arg_roles.iter().find_map(|(index, role)| {
        (*role == tcl_registry::ArgRole::Name)
            .then_some(usize::from(*index) + 1 + layout.member_word)
    })?;
    let name_word = words.words().get(name_at)?;
    let original_name = original_member_input(
        &definition.origin,
        &words,
        segment.span.start(),
        name_at,
        policy,
        context.config,
    )?;
    policy
        .recipe()
        .oo_method_input(original_name.bytes())
        .ok()?;
    let body = deferred_method_body(
        grammar, definition, map, segment, dialect, parameters, context,
    )?;
    let (declaration, _, source) = body.executed_script.as_ref()?;
    Some(SourceReceiverMethodEntry {
        name: std::str::from_utf8(original_name.bytes())
            .unwrap_or_default()
            .to_owned(),
        name_source: name_word.source().clone(),
        receiver,
        exported: grammar.member_default_exported_bytes(original_name.bytes()),
        original_name,
        declaring_class: None,
        declaring_object: None,
        object_body_context: None,
        class_delegate: layout.class_delegate,
        delegate_allocation: None,
        entry: super::super::SourceConstructorEntry {
            declaration: declaration.clone(),
            body: source.clone(),
            formals: body
                .parameters
                .iter()
                .map(|formal| (formal.name.clone(), formal.default.clone()))
                .collect(),
            frame: tcl_names::variable::VariableExecutionFrame::ReceiverMethod {
                identity: super::super::source_activation_name(
                    body.source_origin.as_ref(),
                    &body.identity,
                    body.implementation_generation,
                ),
            },
        },
    })
}

#[derive(Default)]
struct RetainedLifecycleEntries {
    constructor: Option<Arc<super::super::SourceConstructorEntry>>,
    destructor: Option<Arc<super::super::SourceConstructorEntry>>,
}

fn retained_lifecycle_entries(
    grammar: &tcl_registry::definer::DefinitionBodyGrammar,
    definition: &ExecutedScriptSource,
    map: &tcl_lexer::SourceMap<'_>,
    segments: &[tcl_source::segment::SegmentedCommand],
    dialect: Option<tcl_native_model::InvocationDialect>,
    parameters: tcl_dialect::ParameterGrammar,
    context: SourceExecutionContext<'_>,
) -> Option<RetainedLifecycleEntries> {
    let mut selected = RetainedLifecycleEntries::default();
    for segment in segments {
        let words = crate::ir::CommandTokens::from_segmented(map, context.config, segment);
        let values = source_effective_words(words.words(), dialect, None);
        let head = values.first()?.as_registry_word().literal()?;
        let member = grammar.member(head)?;
        let arguments = values
            .iter()
            .skip(1)
            .map(|word| word.as_registry_word().literal())
            .collect::<Option<Vec<_>>>();
        if arguments
            .as_ref()
            .and_then(|arguments| grammar.single_native_inherited_class(member, arguments, dialect))
            .is_some()
        {
            continue;
        }
        if !grammar.preserves_local_constructor_entry(member) {
            return None;
        }
        let constructor = grammar.is_instance_constructor(member);
        if !constructor && !grammar.is_instance_destructor(member) {
            continue;
        }
        let body_at = member.arg_roles.iter().find_map(|(index, role)| {
            (*role == tcl_registry::ArgRole::Body).then_some(usize::from(*index) + 1)
        })?;
        let text = values.get(body_at)?.as_registry_word().literal()?;
        if grammar.native_lifecycle_body_disposition(member, text.as_bytes())
            == Some(tcl_registry::definer::NativeLifecycleBodyDisposition::Removed)
        {
            if constructor {
                selected.constructor = None;
            } else {
                selected.destructor = None;
            }
            continue;
        }
        let body = deferred_method_body(
            grammar, definition, map, segment, dialect, parameters, context,
        )?;
        let (_, _, source) = body.executed_script.as_ref()?;
        let declaration = CommandAllocationSite {
            source: Arc::clone(&definition.origin),
            offset: segment.span.start(),
        };
        let entry = Arc::new(super::super::SourceConstructorEntry {
            declaration,
            body: source.clone(),
            formals: body
                .parameters
                .iter()
                .map(|formal| (formal.name.clone(), formal.default.clone()))
                .collect(),
            frame: tcl_names::variable::VariableExecutionFrame::ReceiverMethod {
                identity: super::super::source_activation_name(
                    body.source_origin.as_ref(),
                    &body.identity,
                    body.implementation_generation,
                ),
            },
        });
        if constructor {
            selected.constructor = Some(entry);
        } else {
            selected.destructor = Some(entry);
        }
    }
    Some(selected)
}

// This is a complete name inventory only for the already-validated, closed
// instance-definition grammar. Visibility is deliberately not a call licence.
fn closed_instance_methods(
    grammar: &tcl_registry::definer::DefinitionBodyGrammar,
    definition: &ExecutedScriptSource,
    map: &tcl_lexer::SourceMap<'_>,
    segments: &[tcl_source::segment::SegmentedCommand],
    dialect: Option<tcl_native_model::InvocationDialect>,
    policy: tcl_syntax::naming::NamePolicyProtocol,
    context: SourceExecutionContext<'_>,
) -> Option<Arc<super::super::BTreeSet<NameBytes>>> {
    use tcl_registry::definer::{BuiltinMethodReceiver, MemberConstructionEffect};
    let mut names = grammar
        .builtin_object_methods
        .iter()
        .filter(|method| method.receiver == BuiltinMethodReceiver::AnyObject)
        .map(|method| NameBytes::from(method.name.as_bytes()))
        .collect::<super::super::BTreeSet<_>>();
    for segment in segments {
        let words = crate::ir::CommandTokens::from_segmented(map, context.config, segment);
        let values = source_effective_words(words.words(), dialect, None);
        let head = values.first()?.as_registry_word().literal()?;
        let member = grammar.member(head)?;
        let arguments = values
            .iter()
            .skip(1)
            .map(|word| word.as_registry_word().literal())
            .collect::<Option<Vec<_>>>();
        if arguments
            .as_ref()
            .and_then(|arguments| grammar.single_native_inherited_class(member, arguments, dialect))
            .is_some()
        {
            continue;
        }
        if grammar.construction_member_effect(member)
            == MemberConstructionEffect::DeferredInstanceMethod
            || dialect.is_some_and(|dialect| {
                grammar
                    .native_deferred_forward_setter(member, dialect, values.len().saturating_sub(1))
                    .is_some()
            })
            || grammar.native_classmethod_declaration(
                member,
                dialect
                    .and_then(tcl_native_model::InvocationDialect::authoring_query)
                    .map(|query| query.with_realm(context.realm())),
            )
        {
            let name_at = member.arg_roles.iter().find_map(|(index, role)| {
                (*role == tcl_registry::ArgRole::Name).then_some(usize::from(*index) + 1)
            })?;
            let name = original_member_input(
                &definition.origin,
                &words,
                segment.span.start(),
                name_at,
                policy,
                context.config,
            )?;
            if grammar
                .unknown_dispatch_method
                .is_some_and(|unknown| unknown.as_bytes() == name.bytes())
            {
                return None;
            }
            policy.recipe().oo_method_input(name.bytes()).ok()?;
            names.insert(NameBytes::from(name.bytes()));
        } else if member.visibility_effect.is_some() {
            // Native visibility metadata cannot add or remove method names.
        } else if member
            .arg_roles
            .iter()
            .any(|(_, role)| *role == tcl_registry::ArgRole::Name)
            || member.all_args_ref.is_some()
            || member.retraction.is_some()
            || member.visibility_effect.is_some()
            || member.slot.is_some()
        {
            return None;
        }
    }
    Some(Arc::new(names))
}

pub(super) fn deferred_method_body(
    grammar: &tcl_registry::definer::DefinitionBodyGrammar,
    definition: &ExecutedScriptSource,
    map: &tcl_lexer::SourceMap<'_>,
    segment: &tcl_source::segment::SegmentedCommand,
    dialect: Option<tcl_native_model::InvocationDialect>,
    parameters: tcl_dialect::ParameterGrammar,
    context: SourceExecutionContext<'_>,
) -> Option<DeferredSourceBody> {
    let config = context.config;
    let words = crate::ir::CommandTokens::from_segmented(map, config, segment);
    let values = source_effective_words(words.words(), dialect, None);
    let head = values.first()?.as_registry_word().literal()?;
    let arguments = values
        .iter()
        .skip(1)
        .map(crate::registry_invocation::EffectiveInvocationWord::literal_bytes)
        .collect::<Option<Vec<_>>>();
    let query = dialect
        .and_then(tcl_native_model::InvocationDialect::authoring_query)
        .map(|query| query.with_realm(context.realm()));
    let layout = arguments.as_ref().and_then(|arguments| {
        grammar.receiver_method_layout_bytes(head.as_bytes(), arguments, query)
    });
    let (member, shift) = layout.map_or((grammar.member(head)?, 0), |layout| {
        (layout.member, layout.member_word)
    });
    let find = |role| {
        member.arg_roles.iter().find_map(|(index, declared)| {
            (*declared == role).then_some(usize::from(*index) + 1 + shift)
        })
    };
    let body_at = find(tcl_registry::ArgRole::Body)?;
    let value = values.get(body_at)?.as_registry_word().literal()?;
    if grammar.native_lifecycle_body_disposition(member, value.as_bytes())
        == Some(tcl_registry::definer::NativeLifecycleBodyDisposition::Removed)
    {
        return None;
    }
    let original_parameters = if let Some(params_at) = find(tcl_registry::ArgRole::ParamList) {
        let dialect = dialect?;
        let topology = super::super::formal_topology::capture(
            definition.origin.source_image(),
            words.words(),
            segment.span.start(),
            params_at,
            config,
            dialect,
            dialect.authored_name_policy()?,
        )?;
        if dialect.parameter_grammar()? != parameters {
            return None;
        }
        Some(topology)
    } else {
        None
    };
    let formals = original_parameters.as_ref().map_or_else(
        Vec::new,
        super::super::formal_topology::OriginalFormalTopology::advisory_parameters,
    );
    let word = words.words().get(body_at)?;
    let site = CommandAllocationSite {
        source: Arc::clone(&definition.origin),
        offset: segment.span.start(),
    };
    let source = ExecutedScriptSource::from_word(site.clone(), body_at - 1, word, value, config);
    let identity = site.variable_storage_identity();
    let body = DeferredSourceBody {
        realm: context.realm(),
        identity,
        implementation_generation: 0,
        // Native member grammar seals a declaration scope here. Actual method
        // publication and receiver allocation retain independent owners.
        implementation_allocation: None,
        source_origin: Some(Arc::clone(&source.origin)),
        executed_script: Some((site, body_at - 1, source.clone())),
        executed_word: Some((Arc::clone(&definition.origin), word.source().span)),
        source: source.try_text().ok()?.to_owned(),
        offset: source.base(),
        namespace: "::".to_owned(),
        namespace_key: context.namespace_identity(),
        event: None,
        receiver_method: true,
        receiver_namespace: (grammar.member_current_namespace()
            == tcl_registry::definer::MemberCurrentNamespace::RuntimeReceiver)
            .then_some(super::super::conditional_body::SourceReceiverNamespace {
                path: grammar.member_body_namespace_path,
                family: grammar.family,
                commands: grammar.profile.receiver_namespace_commands,
            }),
        future_frame: None,
        parameters: formals,
        original_parameters,
        statics: None,
    };
    Some(body)
}
