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

//! Which members of a class definition are closed: forwards, inherited members and method blocks.

use super::super::{
    Arc, CommandAllocationSite, ExecutedScriptSource, ModuleCommandBindings,
    SourceExecutionContext, source_effective_words,
};

pub(in super::super) fn closed_definition_member(
    grammar: &tcl_registry::definer::DefinitionBodyGrammar,
    definition: &ExecutedScriptSource,
    map: &tcl_lexer::SourceMap<'_>,
    segment: &tcl_source::segment::SegmentedCommand,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> bool {
    // naming.tcloo.original-forward-registration-prefix
    // docs/design/analysis/name-resolution-proofs/tcloo-original-forward-registration-prefix.md
    let words = crate::ir::CommandTokens::from_segmented(map, context.config, segment);
    let values = source_effective_words(words.words(), state.baseline.dialect, None);
    let Some(head) = values
        .first()
        .and_then(|word| word.as_registry_word().literal())
    else {
        return false;
    };
    let query = state
        .baseline
        .dialect
        .and_then(tcl_registry::InvocationDialect::authoring_query)
        .map(|query| query.with_realm(context.realm()));
    let arguments = values
        .iter()
        .skip(1)
        .filter_map(crate::registry_invocation::EffectiveInvocationWord::literal_bytes)
        .collect::<Vec<_>>();
    if arguments.len() + 1 != values.len() {
        return false;
    }
    let Some(outer) = grammar.member(head) else {
        return false;
    };
    if let Some(closed) =
        closed_inherited_definition_member(grammar, outer, &values, &arguments, state, context)
    {
        return closed;
    }
    if let Some(block) = class_definition_block(
        grammar,
        definition,
        map,
        segment,
        state.baseline.dialect,
        context,
    ) {
        if !original_definition_member(
            grammar,
            outer,
            tcl_registry::definer::DefinitionReceiver::Instance,
            state,
            context,
        ) {
            return false;
        }
        return closed_class_method_block(grammar, &block, state, context);
    }
    let layout = grammar.receiver_method_layout_bytes(head.as_bytes(), &arguments, query);
    let (member, shift, receiver) = layout.map_or(
        (
            outer,
            0,
            tcl_registry::definer::DefinitionReceiver::Instance,
        ),
        |layout| (layout.member, layout.member_word, layout.worker_receiver),
    );
    if shift != 0
        && !original_definition_member(
            grammar,
            outer,
            tcl_registry::definer::DefinitionReceiver::Instance,
            state,
            context,
        )
    {
        return false;
    }
    if (grammar.manufacture_member_effect(member)
        != tcl_registry::definer::MemberManufactureDispatchEffect::PreservesNativeFactory
        && !layout.is_some_and(|layout| layout.class_delegate))
        || !original_definition_member(grammar, member, receiver, state, context)
    {
        return false;
    }
    if member.kind == tcl_registry::definer::MemberKind::FlagKeyed {
        return valid_property_declarations(
            grammar,
            &arguments[shift..],
            query,
            state.baseline.dialect,
        );
    }
    if shift == 0
        && let Some(closed) = closed_original_forward_member(
            grammar,
            definition,
            member,
            &words,
            (arguments.len(), segment.span.start()),
            state,
            context,
        )
    {
        return closed;
    }
    valid_closed_member_body(grammar, member, &values[shift..], state.baseline.dialect)
}

fn closed_original_forward_member(
    grammar: &tcl_registry::definer::DefinitionBodyGrammar,
    definition: &ExecutedScriptSource,
    member: &tcl_registry::definer::MemberSpec,
    words: &crate::ir::CommandTokens,
    shape: (usize, u32),
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> Option<bool> {
    let dialect = state.baseline.dialect?;
    grammar.native_deferred_forward_setter(member, dialect, shape.0)?;
    Some(
        state
            .baseline
            .execution_name_policy
            .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
            .and_then(|policy| {
                super::super::deferred_forward::original_forward_method(
                    grammar, definition, words, shape.1, dialect, policy, context,
                )
            })
            .is_some(),
    )
}

fn closed_inherited_definition_member(
    grammar: &tcl_registry::definer::DefinitionBodyGrammar,
    outer: &tcl_registry::definer::MemberSpec,
    values: &[crate::registry_invocation::EffectiveInvocationWord],
    arguments: &[&[u8]],
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> Option<bool> {
    let text_arguments = arguments
        .iter()
        .map(|argument| std::str::from_utf8(argument).ok())
        .collect::<Option<Vec<_>>>();
    if text_arguments
        .as_ref()
        .and_then(|arguments| {
            grammar.single_native_inherited_class(outer, arguments, state.baseline.dialect)
        })
        .is_some()
    {
        return Some(
            state
                .closed_inherited_member(grammar, outer, values, context)
                .is_some(),
        );
    }
    None
}

fn valid_property_declarations(
    grammar: &tcl_registry::definer::DefinitionBodyGrammar,
    arguments: &[&[u8]],
    query: Option<tcl_dialect::model::SurfaceQuery<'_>>,
    dialect: Option<tcl_registry::InvocationDialect>,
) -> bool {
    let Some(declarations) = grammar.source_property_declarations_bytes(arguments, query) else {
        return false;
    };
    let Some(dialect) = dialect else {
        return false;
    };
    declarations.iter().all(|declaration| {
        tcl_registry::native_property_lookup::native_property_declaration_validity(
            dialect,
            arguments[declaration.name_index()],
        )
        .is_some_and(|result| result.is_ok())
    })
}

fn valid_closed_member_body(
    grammar: &tcl_registry::definer::DefinitionBodyGrammar,
    member: &tcl_registry::definer::MemberSpec,
    values: &[crate::registry_invocation::EffectiveInvocationWord],
    dialect: Option<tcl_registry::InvocationDialect>,
) -> bool {
    // Class variable declarations are closed native definition metadata;
    // they select no body or constructor callback during declaration.
    if (member.all_args_var || member.visibility_effect.is_some())
        && grammar.preserves_local_constructor_entry(member)
    {
        return true;
    }
    if member
        .arg_roles
        .iter()
        .find_map(|(index, role)| {
            (*role == tcl_registry::ArgRole::Body).then(|| values.get(usize::from(*index) + 1))
        })
        .flatten()
        .and_then(|word| word.as_registry_word().literal())
        .is_some_and(|body| {
            grammar.native_lifecycle_body_disposition(member, body.as_bytes())
                == Some(tcl_registry::definer::NativeLifecycleBodyDisposition::Removed)
        })
    {
        let expected = member
            .arg_roles
            .iter()
            .map(|(index, _)| usize::from(*index) + 2)
            .max();
        return expected == Some(values.len());
    }
    valid_member_formals(member, values, dialect)
}

fn closed_class_method_block(
    grammar: &tcl_registry::definer::DefinitionBodyGrammar,
    block: &ExecutedScriptSource,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> bool {
    let block_map = tcl_lexer::SourceMap::from_image(&block.text).with_base(block.base(), 0, 0);
    tcl_source::segment::segment_commands_image_with_offset_and_config(
        &block.text,
        block.base(),
        context.config,
    )
    .is_some_and(|segments| {
        segments
            .iter()
            .all(|inner| closed_class_method(grammar, &block_map, inner, state, context))
    })
}

fn valid_member_formals(
    member: &tcl_registry::definer::MemberSpec,
    values: &[crate::registry_invocation::EffectiveInvocationWord],
    dialect: Option<tcl_registry::InvocationDialect>,
) -> bool {
    let expected = member
        .arg_roles
        .iter()
        .map(|(index, _)| usize::from(*index) + 2)
        .max()
        .unwrap_or(1);
    !(!member
        .arg_roles
        .iter()
        .any(|(_, role)| *role == tcl_registry::ArgRole::Body)
        || values.len() != expected
        || member.arg_roles.iter().any(|(index, role)| {
            *role == tcl_registry::ArgRole::ParamList
                && values
                    .get(usize::from(*index) + 1)
                    .and_then(crate::registry_invocation::EffectiveInvocationWord::literal_bytes)
                    .is_none_or(|bytes| {
                        dialect
                            .filter(|dialect| dialect.parameter_grammar().is_some())
                            .and_then(|dialect| {
                                dialect.native_name_protocol().or_else(|| {
                                    dialect
                                        .authored_name_policy()
                                        .map(tcl_syntax::naming::NamePolicyProtocol::recipe)
                                })
                            })
                            .is_none_or(|protocol| {
                                !super::super::formal_topology::native_formal_parameter_bytes_valid(
                                    bytes, protocol,
                                )
                            })
                    })
        }))
}

pub(super) fn class_definition_block(
    grammar: &tcl_registry::definer::DefinitionBodyGrammar,
    definition: &ExecutedScriptSource,
    map: &tcl_lexer::SourceMap<'_>,
    segment: &tcl_source::segment::SegmentedCommand,
    dialect: Option<tcl_registry::InvocationDialect>,
    context: SourceExecutionContext<'_>,
) -> Option<ExecutedScriptSource> {
    let words = crate::ir::CommandTokens::from_segmented(map, context.config, segment);
    let values = source_effective_words(words.words(), dialect, None);
    if values.len() != 2 {
        return None;
    }
    let head = values.first()?.as_registry_word().literal()?;
    if !grammar.is_class_receiver_wrapper(grammar.member(head)?) {
        return None;
    }
    let value = values.get(1)?.as_registry_word().literal()?;
    let site = CommandAllocationSite {
        source: Arc::clone(&definition.origin),
        offset: segment.span.start(),
    };
    Some(ExecutedScriptSource::from_word(
        site,
        0,
        words.words().get(1)?,
        value,
        context.config,
    ))
}

pub(in super::super) fn closed_class_method(
    grammar: &tcl_registry::definer::DefinitionBodyGrammar,
    map: &tcl_lexer::SourceMap<'_>,
    segment: &tcl_source::segment::SegmentedCommand,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> bool {
    let words = crate::ir::CommandTokens::from_segmented(map, context.config, segment);
    let values = source_effective_words(words.words(), state.baseline.dialect, None);
    let Some(head) = values
        .first()
        .and_then(|word| word.as_registry_word().literal())
    else {
        return false;
    };
    let Some(member) = grammar.member(head) else {
        return false;
    };
    if !original_definition_member(
        grammar,
        member,
        tcl_registry::definer::DefinitionReceiver::Class,
        state,
        context,
    ) {
        return false;
    }
    let expected = member
        .arg_roles
        .iter()
        .map(|(at, _)| usize::from(*at) + 2)
        .max();
    values.len() == expected.unwrap_or(0)
        && values.iter().all(|word| word.literal_bytes().is_some())
        && valid_member_formals(member, &values, state.baseline.dialect)
}

pub(in super::super) fn original_definition_member(
    grammar: &tcl_registry::definer::DefinitionBodyGrammar,
    member: &tcl_registry::definer::MemberSpec,
    receiver: tcl_registry::definer::DefinitionReceiver,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> bool {
    original_definition_member_target(grammar, member, receiver, state, context).is_some()
}

pub(in super::super) fn original_definition_member_target(
    grammar: &tcl_registry::definer::DefinitionBodyGrammar,
    member: &tcl_registry::definer::MemberSpec,
    receiver: tcl_registry::definer::DefinitionReceiver,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> Option<super::super::SourceCommandTarget> {
    let query = state
        .baseline
        .dialect
        .and_then(tcl_registry::InvocationDialect::authoring_query)
        .map(|query| query.with_realm(context.realm()));
    let mut lookup = grammar.definition_member_lookup_for_receiver(member, receiver, query)?;
    if receiver == tcl_registry::definer::DefinitionReceiver::Instance
        && !grammar.property_accessor_methods.is_empty()
    {
        let support = context
            .registry
            .native_class_factory_recipe(
                "oo::configurable",
                state.baseline.dialect?,
                context.realm(),
            )?
            .support()?;
        lookup.namespace = support.definition_namespace(receiver);
        if member.kind == tcl_registry::definer::MemberKind::FlagKeyed {
            lookup.implementation =
                tcl_syntax::naming::rooted_member_key(lookup.namespace, member.keyword);
        }
    }
    let namespace_key = state.namespace_for_rooted_operand(lookup.namespace)?;
    let policy = state.baseline.execution_name_policy?.native_recipe()?;
    let target = state.original_registry_metadata_target(&namespace_key, member.keyword, policy)?;
    let target = super::super::SourceCommandTarget {
        runtime_implementation_generation: state
            .runtime_implementation_generation(target.token.as_ref()),
        command: target.command,
        prepended: target.prepended,
        original_prepended: None,
        logical_original_prefix: None,
        registry_backed: target.registry_backed,
        kind: target.kind,
        identity: target.token,
        implementation_generation: target.implementation_generation,
        implementation_allocation: target.implementation_allocation,
    };
    Some(target).filter(|target| {
        target.registry_backed
            && target.prepended.is_empty()
            && target.registry_identity().is_some_and(|identity| {
                tcl_registry::command_descriptor_labels_match(identity, &lookup.implementation)
            })
            && target.implementation_generation == 0
            && !state.tainted_object_dispatch.contains("*")
            && !state.tainted_object_dispatch.contains(&target.command)
            && !state.source_step_observed()
            && !state.source_execution_observed(target.identity.as_ref())
    })
}
