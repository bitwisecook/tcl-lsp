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

//! Which semantic operation and merged semantics the command, subcommand and form descriptors select for one invocation.

use super::invocation_options::invocation_options;
use super::source_definitions::NamedObjectFactory;
use super::{InvocationAvailability, InvocationSemantics};
use crate::command_table::CommandTableEffect;
use crate::completion::CompletionDescriptor;
use crate::dispatch_stability::ResolvedDispatchDependencies;
use crate::forms::CommandForm;
use crate::hooks::{CodegenHookId, InlineCodegenHookId, LoweringHookId};
use crate::intrinsic::IntrinsicId;
use crate::option_effect::OptionEffectScope;
use crate::semantic_operation::SemanticOperationId;
use crate::spec::{CommandSpec, SubCommand};
use crate::stamp_window::StampSelection;
use crate::state_transition::ResolvedStateTransitions;
use crate::traits::Traits;
use crate::world_effect::ResolvedWorldEffects;
use tcl_dialect::model::SurfaceQuery;

pub(crate) fn descriptor_operation(
    semantic: Option<SemanticOperationId>,
    lowering: Option<LoweringHookId>,
    codegen: Option<CodegenHookId>,
    inline_codegen: Option<InlineCodegenHookId>,
) -> Option<SemanticOperationId> {
    level_operation(
        StampSelection::stated(semantic),
        lowering,
        StampSelection::stated(codegen),
        StampSelection::stated(inline_codegen),
    )
    .stamp()
}

/// What one level of descriptors — a form, a subcommand, or the command —
/// says its semantic operation is, from the stamps it carries at the point
/// asked about.
///
/// The order is the one the unversioned fields always had: a stated operation,
/// else the structured lowering, else the intrinsic an inline hook names, else
/// the one a codegen hook names. A level that declines on any of the stamps
/// that could decide it declines as a whole — its operation is not known — and
/// the level above does not answer in its place.
fn level_operation(
    semantic: StampSelection<SemanticOperationId>,
    lowering: Option<LoweringHookId>,
    codegen: StampSelection<CodegenHookId>,
    inline_codegen: StampSelection<InlineCodegenHookId>,
) -> StampSelection<SemanticOperationId> {
    let intrinsic = || {
        match inline_codegen {
            StampSelection::Decline => return StampSelection::Decline,
            StampSelection::Stamp(hook) => {
                if let Some(id) = IntrinsicId::from_legacy_inline_codegen(hook) {
                    return StampSelection::Stamp(SemanticOperationId::Intrinsic(id));
                }
            }
            StampSelection::Inherit => {}
        }
        match codegen {
            StampSelection::Decline => StampSelection::Decline,
            StampSelection::Stamp(hook) => StampSelection::stated(
                IntrinsicId::from_legacy_codegen(hook).map(SemanticOperationId::Intrinsic),
            ),
            StampSelection::Inherit => StampSelection::Inherit,
        }
    };
    semantic
        .or(StampSelection::stated(
            lowering.map(SemanticOperationId::StructuredLowering),
        ))
        .or(intrinsic())
}

fn form_operation(form: Option<&CommandForm>) -> StampSelection<SemanticOperationId> {
    form.map_or(StampSelection::Inherit, |form| {
        level_operation(
            StampSelection::stated(form.semantic_operation),
            form.lowering_hook,
            StampSelection::stated(form.codegen_hook),
            StampSelection::Inherit,
        )
    })
}

fn subcommand_operation(
    sub: Option<&SubCommand>,
    query: Option<&SurfaceQuery<'_>>,
) -> StampSelection<SemanticOperationId> {
    sub.map_or(StampSelection::Inherit, |sub| {
        level_operation(
            sub.semantic_operation_selection(query),
            sub.lowering_hook,
            sub.codegen_hook_selection(query),
            sub.inline_codegen_hook_selection(query),
        )
    })
}

fn command_operation(
    spec: &CommandSpec,
    query: Option<&SurfaceQuery<'_>>,
) -> StampSelection<SemanticOperationId> {
    level_operation(
        spec.semantic_operation_selection(query),
        spec.lowering_hook,
        spec.codegen_hook_selection(query),
        spec.inline_codegen_hook_selection(query),
    )
}

pub(crate) fn resolved_operation(
    spec: &CommandSpec,
    sub: Option<&SubCommand>,
    form: Option<&CommandForm>,
    query: Option<&SurfaceQuery<'_>>,
) -> SemanticOperationId {
    form_operation(form)
        .or(subcommand_operation(sub, query))
        .or(command_operation(spec, query))
        .stamp()
        .unwrap_or(SemanticOperationId::Invoke)
}

// This is the single exhaustive projection from three nested registry owners
// into one semantic view; splitting it would duplicate the inheritance rules.
#[allow(clippy::too_many_lines)]
pub(super) fn resolve_invocation_semantics<'r, 'w>(
    spec: &'r CommandSpec,
    sub: Option<&'r SubCommand>,
    form: Option<&'r CommandForm>,
    inherit_command: bool,
    dialect: Option<crate::InvocationDialect>,
    query: Option<&SurfaceQuery<'_>>,
) -> InvocationSemantics<'r, 'w> {
    let (arg_roles, arg_role_resolver, arg_role_resolver_roles) = match form {
        Some(form) => (form.arg_roles, None, &[][..]),
        None => match sub {
            Some(sub) => (
                sub.arg_roles,
                sub.arg_role_resolver,
                sub.arg_role_resolver_roles,
            ),
            None => (
                spec.arg_roles,
                spec.arg_role_resolver,
                spec.arg_role_resolver_roles,
            ),
        },
    };
    let inherited_traits = if inherit_command {
        spec.traits
    } else {
        Traits::empty()
    } | sub.map_or_else(Traits::empty, SubCommand::semantic_traits);
    let inherited_side_effects = sub
        .filter(|sub| !sub.side_effects.is_unknown())
        .map_or_else(
            || {
                if inherit_command {
                    spec.side_effects
                } else {
                    crate::knowledge::Knowledge::Unknown
                }
            },
            |sub| sub.side_effects,
        );
    let operation = if inherit_command {
        resolved_operation(spec, sub, form, query)
    } else {
        form_operation(form)
            .or(subcommand_operation(sub, query))
            .stamp()
            .unwrap_or(SemanticOperationId::Invoke)
    };
    let codegen_hook = StampSelection::stated(form.and_then(|form| form.codegen_hook))
        .or(sub.map_or(StampSelection::Inherit, |sub| {
            sub.codegen_hook_selection(query)
        }))
        .or(if inherit_command {
            spec.codegen_hook_selection(query)
        } else {
            StampSelection::Inherit
        })
        .stamp();
    InvocationSemantics {
        script_metadata: crate::selected_script_timing::SelectedScriptMetadata {
            command: spec,
            subcommand: sub,
        },
        named_object_factory: if inherit_command && sub.is_none() {
            spec.creates_instance_at
                .zip(spec.object_class)
                .map(|(argument, class)| NamedObjectFactory {
                    argument,
                    class_name: class.class_name,
                })
        } else {
            None
        },
        operation,
        codegen_hook,
        completion: form
            .and_then(|form| form.completion)
            .or(sub.and_then(|sub| sub.completion))
            .or(inherit_command.then_some(spec.completion).flatten())
            .unwrap_or(CompletionDescriptor::CONSERVATIVE),
        result_stability: form
            .and_then(|form| form.result_stability)
            .or(sub.and_then(|sub| sub.result_stability))
            .or(inherit_command.then_some(spec.result_stability).flatten())
            .unwrap_or_default(),
        native_result: form
            .and_then(|form| form.native_result)
            .or(sub.and_then(|sub| sub.native_result))
            .or(inherit_command.then_some(spec.native_result).flatten()),
        representation_effect: form
            .and_then(|form| form.representation_effect)
            .or(sub.and_then(|sub| sub.representation_effect))
            .or(inherit_command
                .then_some(spec.representation_effect)
                .flatten())
            .unwrap_or_default(),
        arg_types: form
            .and_then(|form| form.arg_types)
            .unwrap_or_else(|| sub.map_or(spec.arg_types, |sub| sub.arg_types)),
        traits: form
            .and_then(|form| form.traits)
            .unwrap_or(inherited_traits),
        taint_source: inherit_command.then_some(spec.taint_source).flatten(),
        taint_transform: crate::taint::SelectedTaintTransform::for_descriptors(
            spec,
            sub,
            inherit_command,
        ),
        mutator: form
            .and_then(|form| form.mutator)
            .unwrap_or_else(|| sub.is_some_and(|sub| sub.mutator)),
        arity: crate::native_list_assignment::operation_arity(operation, dialect).unwrap_or_else(
            || {
                form.map_or_else(
                    || sub.map_or(spec.arity, |sub| sub.arity),
                    |form| form.arity,
                )
            },
        ),
        argument_offset: usize::from(sub.is_some()),
        arg_roles,
        arg_role_resolver,
        clause_grammar: if form.is_some() {
            None
        } else {
            sub.map_or(spec.clause_grammar, |sub| sub.clause_grammar)
        },
        option_scope: sub.map_or(
            OptionEffectScope {
                families: spec.option_effect_families,
                reserved_trailing_words: spec.reserved_trailing_words,
                prefix_matching: spec.prefix_matching,
                parent_surface: spec.surface,
            },
            |sub| OptionEffectScope {
                families: sub.option_effect_families,
                reserved_trailing_words: 0,
                prefix_matching: sub.prefix_matching,
                parent_surface: sub.surface.or(spec.surface),
            },
        ),
        arg_role_count_resolver: if form.is_some() {
            None
        } else {
            sub.map_or(spec.arg_role_count_resolver, |sub| {
                sub.arg_role_count_resolver
            })
        },
        arg_role_layout_resolver: if form.is_some() {
            None
        } else {
            sub.map_or(spec.arg_role_layout_resolver, |sub| {
                sub.arg_role_layout_resolver
            })
        },
        arg_role_resolver_roles,
        repeated_args: sub.map_or(spec.repeated_args, |sub| sub.repeated_args),
        options: invocation_options(spec, sub, form, InvocationAvailability::default()),
        return_type: form
            .and_then(|form| form.return_type)
            .unwrap_or_else(|| sub.map_or(spec.return_type, |sub| sub.return_type)),
        procedure_definition: inherit_command
            .then_some(spec.procedure_definition)
            .flatten(),
        native_compilation: sub
            .and_then(|sub| sub.native_compilation)
            .or(inherit_command.then_some(spec.native_compilation).flatten()),
        successful_handler: form
            .and_then(|form| form.successful_handler)
            .or(sub.and_then(|sub| sub.successful_handler))
            .or(inherit_command.then_some(spec.successful_handler).flatten()),
        variable_receivers: form
            .and_then(|form| form.variable_receivers)
            .or(sub.and_then(|sub| sub.variable_receivers))
            .or(inherit_command.then_some(spec.variable_receivers).flatten()),
        byte_array_effect: form
            .and_then(|form| form.byte_array_effect)
            .unwrap_or_else(|| sub.map_or(spec.byte_array_effect, |sub| sub.byte_array_effect)),
        byte_array_payload: spec.byte_array_payload,
        safe_on_uninit: form
            .and_then(|form| form.safe_on_uninit)
            .unwrap_or_else(|| {
                sub.and_then(|sub| sub.safe_on_uninit)
                    .or(inherit_command.then_some(spec.safe_on_uninit).flatten())
            }),
        var_write_typing: sub.map_or(spec.var_write_typing, |sub| sub.var_write_typing),
        return_elements: sub.map_or(spec.return_elements, |sub| sub.return_elements),
        var_elements_effect: form
            .and_then(|form| form.var_elements_effect)
            .unwrap_or_else(|| sub.map_or(spec.var_elements_effect, |sub| sub.var_elements_effect)),
        frame_effect: inherit_command.then_some(spec.frame_effect).flatten(),
        body_kind: sub.map_or(spec.body_kind, |sub| sub.body_kind),
        body_execution: sub
            .and_then(|sub| sub.body_execution)
            .or(spec.body_execution),
        body_interpreter: sub.map_or(spec.body_interpreter, |sub| sub.body_interpreter),
        analyser_hook: sub
            .and_then(|sub| sub.analyser_hook)
            .or(inherit_command.then_some(spec.analyser_hook).flatten()),
        lowering_hook: form
            .and_then(|form| form.lowering_hook)
            .or(sub.and_then(|sub| sub.lowering_hook))
            .or(inherit_command.then_some(spec.lowering_hook).flatten()),
        side_effects: form
            .and_then(|form| form.side_effects)
            .unwrap_or(inherited_side_effects),
        world_effects: ResolvedWorldEffects {
            command: inherit_command.then_some(spec.world_effects).flatten(),
            subcommand: sub.and_then(|sub| sub.world_effects),
            form: form.and_then(|form| form.world_effects),
        },
        state_transitions: ResolvedStateTransitions {
            // A `SpecTcl` pack cannot supply a Rust resolver, so it declares
            // a command-table mutation with the one-word
            // `command_table_effect` selector instead. Resolve it to the
            // very stock descriptor a shipped spec names, so the pack
            // shorthand and the shipped declaration share one transition resolver.
            command: inherit_command
                .then(|| {
                    spec.state_transitions.or_else(|| {
                        spec.command_table_effect
                            .map(CommandTableEffect::transitions)
                    })
                })
                .flatten(),
            subcommand: sub.and_then(|sub| {
                sub.state_transitions.or_else(|| {
                    sub.command_table_effect
                        .map(CommandTableEffect::transitions)
                })
            }),
            form: form.and_then(|form| form.state_transitions),
        },
        dispatch_dependencies: ResolvedDispatchDependencies {
            command: inherit_command
                .then_some(spec.dispatch_dependencies)
                .flatten(),
            subcommand: sub.and_then(|sub| sub.dispatch_dependencies),
            form: form.and_then(|form| form.dispatch_dependencies),
        },
        value: crate::value_transfer::declaration::resolve_semantics_scoped(
            spec,
            sub,
            form,
            inherit_command,
        ),
        literal_argument_validator: form
            .and_then(|form| form.literal_argument_validator)
            .or(sub.and_then(|sub| sub.literal_argument_validator))
            .or(inherit_command
                .then_some(spec.literal_argument_validator)
                .flatten()),
    }
}
