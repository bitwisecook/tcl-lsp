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

//! Target-neutral resolution of one Tcl command invocation.
//!
//! [`ResolvedInvocation`] is the common compiler-facing projection of the
//! registry's command, subcommand, and form metadata. It deliberately carries
//! Tcl semantics and source-word facts, but not a backend code-generation
//! hook: target selection happens after common analysis has established that a
//! semantic operation is legal.

use crate::arg_role::ArgRole;
use crate::arity::Arity;
use crate::body_kind::{BodyInterpreter, BodyKind};
use crate::command_table::CommandTableEffect;
use crate::completion::CompletionDescriptor;
use crate::dispatch_stability::{DispatchDependencies, ResolvedDispatchDependencies};
use crate::forms::CommandForm;
use crate::frame_effect::{FrameEffectSpec, FrameLevel};
use crate::hooks::{CodegenHookId, InlineCodegenHookId, LoweringHookId};
use crate::hover::OptionSpec;
use crate::intrinsic::IntrinsicId;
use crate::invocation_words::{
    CommandPrefixArguments, InvocationWord, InvocationWordKind, InvocationWords,
};
use crate::literal_validation::{LiteralArgumentValidation, LiteralArgumentValidator};
use crate::option_effect::{OptionEffectScope, OptionEffects};
use crate::representation::RepresentationEffect;
use crate::result_stability::ResultStability;
use crate::semantic_operation::SemanticOperationId;
use crate::side_effects::SideEffect;
use crate::spec::{
    ArgRoleCountResolver, ArgRoleLayoutResolver, ArgRoleResolver, ArgRoleResolverInput,
    CommandSpec, SubCommand,
};
use crate::spec::{CaseInvocation, InlineCaseClause};
use crate::stamp_window::StampSelection;
use crate::state_transition::{
    ResolvedStateTransitions, StateTransitionKnowledge, StateTransitions,
};
use crate::traits::Traits;
use crate::types::{ReturnElements, TclType, VarElementsEffect, VarWriteTyping};
use crate::value_transfer::inputs::OperandId;
use crate::world_effect::TransitionEffectCoverages;
use crate::world_effect::{EffectFootprint, ResolvedWorldEffects};
use tcl_dialect::model::{SpecSurface, SurfaceQuery};

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
fn resolve_invocation_semantics<'r, 'w>(
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
    let inherited_side_effects = sub.filter(|sub| !sub.side_effects.is_empty()).map_or_else(
        || {
            if inherit_command {
                spec.side_effects
            } else {
                &[]
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
            // shorthand and the shipped declaration produce one transition
            // vocabulary through one resolver (ledger C8).
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

/// A subcommand matched by [`crate::CommandRegistry::resolve_invocation`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResolvedSubcommand<'w> {
    /// The first argument word exactly as supplied by the caller.
    pub spelling: &'w str,
    /// The registry's canonical subcommand name.
    pub canonical_name: &'static str,
}

/// The typed outcome of resolving the first post-head word as a subcommand.
///
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SubcommandResolutionKind {
    /// The source word equals the canonical subcommand spelling.
    Exact,
    /// The source word is a registry-approved unique prefix.
    UniquePrefix,
    /// The source word prefixes several registry subcommands.
    Ambiguous,
    /// The source word names no registry subcommand.
    Unknown,
    /// The source word is not a known literal, so no subcommand lookup was
    /// attempted.
    Indeterminate,
}

/// The subcommand-resolution outcome for a command invocation.
///
/// A command without subcommands, or an invocation with no first argument to
/// resolve, is [`Self::NotApplicable`].  Unknown and ambiguous words remain
/// visible to common analysis rather than being silently treated as an
/// ordinary command form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SubcommandResolution<'w> {
    /// There is no subcommand table or no first post-head word to query.
    NotApplicable,
    /// The source word exactly matches a declared subcommand.
    Exact(ResolvedSubcommand<'w>),
    /// The source word uniquely prefixes a declared subcommand.
    UniquePrefix(ResolvedSubcommand<'w>),
    /// The source word prefixes several declared subcommands.
    Ambiguous {
        /// The source word that was ambiguous.
        spelling: &'w str,
    },
    /// The source word names no declared subcommand.
    Unknown {
        /// The source word that was not in the table.
        spelling: &'w str,
    },
    /// The first post-head word is substituted, expanded, or opaque, so its
    /// runtime subcommand value cannot be selected from registry metadata.
    ///
    /// This outcome intentionally exposes only the source-word category, not
    /// a raw spelling that a consumer might confuse with the evaluated value.
    Indeterminate {
        /// Why the subcommand word cannot be looked up statically.
        word_kind: InvocationWordKind,
    },
}

impl<'w> SubcommandResolution<'w> {
    /// The resolution kind when a word was looked up in a subcommand table.
    #[must_use]
    pub const fn kind(self) -> Option<SubcommandResolutionKind> {
        match self {
            Self::NotApplicable => None,
            Self::Exact(_) => Some(SubcommandResolutionKind::Exact),
            Self::UniquePrefix(_) => Some(SubcommandResolutionKind::UniquePrefix),
            Self::Ambiguous { .. } => Some(SubcommandResolutionKind::Ambiguous),
            Self::Unknown { .. } => Some(SubcommandResolutionKind::Unknown),
            Self::Indeterminate { .. } => Some(SubcommandResolutionKind::Indeterminate),
        }
    }

    /// The matched canonical subcommand, for exact and unique-prefix cases.
    #[must_use]
    pub const fn resolved(self) -> Option<ResolvedSubcommand<'w>> {
        match self {
            Self::Exact(subcommand) | Self::UniquePrefix(subcommand) => Some(subcommand),
            Self::NotApplicable
            | Self::Ambiguous { .. }
            | Self::Unknown { .. }
            | Self::Indeterminate { .. } => None,
        }
    }

    /// Whether this outcome selected a subcommand descriptor.
    #[must_use]
    pub const fn is_resolved(self) -> bool {
        self.resolved().is_some()
    }

    /// Materialise this borrowed outcome for an owned consumer.
    #[must_use]
    pub fn into_owned(self) -> OwnedSubcommandResolution {
        match self {
            Self::NotApplicable => OwnedSubcommandResolution::NotApplicable,
            Self::Exact(subcommand) => OwnedSubcommandResolution::Exact {
                spelling: subcommand.spelling.to_owned(),
                canonical_name: subcommand.canonical_name.to_owned(),
            },
            Self::UniquePrefix(subcommand) => OwnedSubcommandResolution::UniquePrefix {
                spelling: subcommand.spelling.to_owned(),
                canonical_name: subcommand.canonical_name.to_owned(),
            },
            Self::Ambiguous { spelling } => OwnedSubcommandResolution::Ambiguous {
                spelling: spelling.to_owned(),
            },
            Self::Unknown { spelling } => OwnedSubcommandResolution::Unknown {
                spelling: spelling.to_owned(),
            },
            Self::Indeterminate { word_kind } => {
                OwnedSubcommandResolution::Indeterminate { word_kind }
            }
        }
    }
}

/// The owned form of [`SubcommandResolution`] retained by executable IR.
///
/// The non-resolved outcomes remain distinct so a later analysis can explain
/// why no subcommand-specific fact was available rather than treating every
/// case as an absent subcommand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnedSubcommandResolution {
    /// There is no subcommand table or no first post-head word to query.
    NotApplicable,
    /// A literal word exactly matched a declared subcommand.
    Exact {
        /// Literal source spelling.
        spelling: String,
        /// Registry canonical subcommand identity.
        canonical_name: String,
    },
    /// A literal word uniquely prefixed a declared subcommand.
    UniquePrefix {
        /// Literal source spelling.
        spelling: String,
        /// Registry canonical subcommand identity.
        canonical_name: String,
    },
    /// A literal word prefixed several declared subcommands.
    Ambiguous {
        /// Literal source spelling.
        spelling: String,
    },
    /// A literal word matched no declared subcommand.
    Unknown {
        /// Literal source spelling.
        spelling: String,
    },
    /// A substituted, expanded, or opaque word could not be looked up.
    Indeterminate {
        /// The source-word category that prevented lookup.
        word_kind: InvocationWordKind,
    },
}

impl OwnedSubcommandResolution {
    /// Return the canonical subcommand identity for resolved outcomes.
    #[must_use]
    pub fn canonical_name(&self) -> Option<&str> {
        match self {
            Self::Exact { canonical_name, .. } | Self::UniquePrefix { canonical_name, .. } => {
                Some(canonical_name)
            }
            Self::NotApplicable
            | Self::Ambiguous { .. }
            | Self::Unknown { .. }
            | Self::Indeterminate { .. } => None,
        }
    }
}

/// Why source-aware registry resolution could not select an invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InvocationResolutionUnresolved<'w> {
    /// The command head is computed, expanded, or opaque, so it cannot be
    /// looked up in a literal registry table.
    ComputedHead {
        /// The source-word category that prevented lookup.
        word_kind: InvocationWordKind,
    },
    /// The command head was literal but no registry entry is available for
    /// the active dialect.
    UnknownLiteralHead {
        /// Literal source spelling.
        spelling: &'w str,
    },
}

/// The typed outcome of source-aware invocation resolution.
///
/// Exactly one of the two private fields is present. This deliberately uses a
/// compact result structure rather than boxing a large borrowed
/// [`ResolvedInvocation`] in an enum variant: structured resolution remains
/// allocation-free until an owned facts projection is requested.
#[derive(Debug, Clone, Copy)]
pub struct StructuredInvocationResolution<'r, 'w> {
    pub(crate) invocation: Option<ResolvedInvocation<'r, 'w>>,
    pub(crate) unresolved: Option<InvocationResolutionUnresolved<'w>>,
}

impl<'r, 'w> StructuredInvocationResolution<'r, 'w> {
    pub(crate) const fn from_unresolved(reason: InvocationResolutionUnresolved<'w>) -> Self {
        Self {
            invocation: None,
            unresolved: Some(reason),
        }
    }

    /// Return the borrowed resolved invocation, if registry selection
    /// succeeded.
    #[must_use]
    pub const fn resolved(self) -> Option<ResolvedInvocation<'r, 'w>> {
        self.invocation
    }

    /// Return the typed unresolved reason, if registry selection failed.
    #[must_use]
    pub const fn unresolved(self) -> Option<InvocationResolutionUnresolved<'w>> {
        self.unresolved
    }
}

/// The semantic portion of an invocation form.
///
/// This is intentionally a projection rather than a borrowed
/// [`CommandForm`].  `CommandForm` still carries legacy backend hook fields;
/// exposing it from the common API would let a consumer accidentally couple
/// semantic resolution to a target emitter.
#[derive(Debug, Clone, Copy)]
pub struct ResolvedForm<'r> {
    /// Registry-stable form name, such as `"implicit"` or `"flat_path"`.
    pub name: &'static str,
    /// The form's arity after the command head or subcommand word.
    pub arity: Arity,
    /// Form-local static argument roles.
    pub arg_roles: &'r [(u8, ArgRole)],
    /// Form-local options.  Shared options remain in
    /// [`InvocationOptions::base`].
    pub options: &'r [OptionSpec],
}

/// Availability selected once at semantic invocation ingress.
#[derive(Debug, Clone, Copy, Default)]
pub struct InvocationAvailability<'r> {
    /// Actual requested core/package surface, independent of operand syntax.
    pub query: Option<SurfaceQuery<'r>>,
    /// Explicit registry/profile floor of the owning package, never inferred from its name.
    pub package_version: Option<&'r str>,
}

/// Option descriptors applicable to a resolved invocation.
///
/// Command and subcommand options remain separate from form-local options so
/// the resolver remains allocation-free.  Consumers must consult both slices;
/// this mirrors the registry keyword-table construction.
#[derive(Debug, Clone, Copy)]
pub struct InvocationOptions<'r, 'q> {
    /// Ingress-selected option availability.
    pub availability: InvocationAvailability<'q>,
    /// Surface inherited by shared command/subcommand options.
    pub parent_surface: Option<&'static [SpecSurface]>,
    /// Surface inherited by form-local options.
    pub form_surface: Option<&'static [SpecSurface]>,
    /// The selected option table's actual prefix policy.
    pub prefix_matching: crate::abbrev::PrefixMatching,
    /// Authored constructor or lifecycle operands preceding the option prefix.
    pub positional_prefix_words: usize,
    /// Native operands excluded from the leading option scan.
    pub reserved_trailing_words: usize,
    /// Authored case grammar's conditional optionless layout.
    pub case_list: Option<&'r crate::spec::CaseListSpec>,
    /// Options declared by the command or resolved subcommand.
    pub base: &'r [OptionSpec],
    /// Options declared only by the matched form.
    pub form: &'r [OptionSpec],
}

/// One option or terminator selected in a proved invocation prefix.
#[derive(Debug, Clone)]
pub struct InvocationOptionOccurrence<'r> {
    /// Post-head ordinal within the argument slice supplied to the owner.
    pub argument_index: usize,
    /// Selected descriptor; `None` is the declared `--` terminator.
    pub option: Option<&'r OptionSpec>,
    /// Exact value positions, empty for flags and the terminator.
    pub values: std::ops::Range<usize>,
}

impl<'r, 'q> InvocationOptions<'r, 'q> {
    /// Keyword vocabulary of the same selected options and prefix policy.
    /// Actual surface, package floor and form inheritance remain on this
    /// metadata; option positions and value widths require `prefix_occurrences`.
    /// This supplies no handler, successful dispatch or rewrite permission.
    #[must_use]
    pub fn keyword_table(self) -> crate::abbrev::KeywordTable<'static> {
        // naming.minifier.complete-logical-metadata
        // docs/design/analysis/name-resolution-proofs/minifier-complete-logical-metadata.md
        crate::spec::keyword_table_for_available_options(self.available(), self.prefix_matching)
    }

    /// Project the selected leading option grammar without exposing computed
    /// values as literals. Availability, prefix ambiguity, reserved operands
    /// and value widths share the same owners as argument-role resolution.
    #[must_use]
    pub fn prefix_occurrences(
        self,
        arguments: crate::InvocationArguments<'_>,
    ) -> Option<Vec<InvocationOptionOccurrence<'r>>> {
        let end = self.leading_word_count(arguments)?;
        let options = self.available().collect::<Vec<_>>();
        let mut index = self.positional_prefix_words;
        let mut occurrences = Vec::new();
        while index < end {
            let word = arguments.literal_at(index)?;
            if word == "--" {
                occurrences.push(InvocationOptionOccurrence {
                    argument_index: index,
                    option: None,
                    values: index + 1..index + 1,
                });
                break;
            }
            let option = crate::spec::resolve_available_option_prefix_with(
                &options,
                word,
                self.prefix_matching,
            )?;
            let first = index.checked_add(1)?;
            let next =
                first.checked_add(option.value_word_count_for_arguments(arguments, index)?)?;
            if next > end {
                return None;
            }
            occurrences.push(InvocationOptionOccurrence {
                argument_index: index,
                option: Some(option),
                values: first..next,
            });
            index = next;
        }
        Some(occurrences)
    }

    /// Selected option-value roles, relative to this argument slice. Fixed
    /// value widths preserve positions even when the value bytes are unknown;
    /// unknown option selection or width withdraws the complete projection.
    pub(crate) fn value_roles(
        self,
        arguments: crate::InvocationArguments<'_>,
    ) -> Option<Vec<(usize, ArgRole)>> {
        let options = self.available().collect::<Vec<_>>();
        if !options
            .iter()
            .any(|option| option.value_role().is_some() || option.value_also_role().is_some())
        {
            return Some(Vec::new());
        }
        let end = self.leading_word_count(arguments)?;
        let mut index = self.positional_prefix_words;
        let mut roles = Vec::new();
        while index < end {
            let word = arguments.literal_at(index)?;
            if word == "--" {
                break;
            }
            let option = crate::spec::resolve_available_option_prefix_with(
                &options,
                word,
                self.prefix_matching,
            )?;
            let count = option.value_word_count_for_arguments(arguments, index)?;
            for role in [option.value_role(), option.value_also_role()]
                .into_iter()
                .flatten()
            {
                roles.extend((index + 1..index + 1 + count).map(|argument| (argument, role)));
            }
            index += 1 + count;
        }
        Some(roles)
    }

    fn diagnostic_option(
        self,
        word: &str,
    ) -> Result<&'r OptionSpec, crate::abbrev::KeywordMatch<'static>> {
        if let Some(exact) = self
            .base
            .iter()
            .chain(self.form)
            .find(|option| option.matches(word))
        {
            return Ok(exact);
        }
        let available = self.available().collect::<Vec<_>>();
        let table = crate::spec::keyword_table_for_available_options(
            available.iter().copied(),
            self.prefix_matching,
        );
        match table.resolve(word) {
            crate::abbrev::KeywordMatch::Unique(name) => available
                .into_iter()
                .find(|option| option.name == name)
                .ok_or(crate::abbrev::KeywordMatch::Unknown),
            other => Err(other),
        }
    }

    fn diagnostic_scan(
        self,
        arguments: crate::InvocationArguments<'_>,
        offset: usize,
        placement: crate::OptionPlacement,
    ) -> Option<AuthoredSourceOptionScan<'r>> {
        let count = arguments.exact_argv_len()?;
        let reserved = self.reserved_word_count(arguments)?;
        let mut scan = AuthoredSourceOptionScan {
            options: Vec::new(),
            accepts_terminator: self.available().any(|option| option.name == "--"),
            subcommands: Vec::new(),
            boundary: AuthoredSourceOptionBoundary::End,
        };
        let end = count.saturating_sub(reserved);
        let mut index = self.positional_prefix_words;
        while index < end {
            let argument = offset.checked_add(index)?;
            let Some(word) = arguments.literal_at(index) else {
                scan.boundary = AuthoredSourceOptionBoundary::Dynamic(argument);
                break;
            };
            if word == "--" && scan.accepts_terminator {
                scan.boundary = AuthoredSourceOptionBoundary::Terminator(argument);
                break;
            }
            if !word.starts_with('-') || word == "-" {
                if placement == crate::OptionPlacement::Anywhere {
                    index += 1;
                    continue;
                }
                scan.boundary = AuthoredSourceOptionBoundary::Positional(argument);
                break;
            }
            let option = match self.diagnostic_option(word) {
                Ok(option) => option,
                Err(crate::abbrev::KeywordMatch::Ambiguous(candidates)) => {
                    scan.boundary = AuthoredSourceOptionBoundary::Ambiguous {
                        argument,
                        candidates,
                    };
                    break;
                }
                Err(_) => {
                    scan.boundary = AuthoredSourceOptionBoundary::Unknown(argument);
                    break;
                }
            };
            let surface = if self
                .form
                .iter()
                .any(|candidate| std::ptr::eq(candidate, option))
            {
                self.form_surface
            } else {
                self.parent_surface
            };
            let next = option
                .value_word_count_for_arguments(arguments, index)
                .and_then(|width| index.checked_add(1)?.checked_add(width))
                .filter(|next| *next <= end);
            scan.options.push(AuthoredSourceOption {
                argument,
                option,
                available: option.supports_dialect(self.availability.query, surface)
                    && option.available_for_version(self.availability.package_version),
                surface: option.surface.or(surface),
                values: next
                    .and_then(|next| Some(argument.checked_add(1)?..offset.checked_add(next)?)),
            });
            let Some(next) = next else {
                scan.boundary = AuthoredSourceOptionBoundary::Indeterminate;
                break;
            };
            index = next;
        }
        Some(scan)
    }

    fn reserved_word_count(self, arguments: crate::InvocationArguments<'_>) -> Option<usize> {
        match self.case_list {
            Some(case) => case.option_scan_reserved_for_arguments(
                arguments,
                self.availability.query,
                self.reserved_trailing_words,
            ),
            None => Some(self.reserved_trailing_words),
        }
    }

    /// Possible option words under the selected grammar. Unknown operands
    /// branch over admitted option widths; they do not supply values or prove
    /// that any branch reaches a handler. Ordinals are relative to this slice.
    fn possible_option_arguments(
        self,
        arguments: crate::InvocationArguments<'_>,
        placement: crate::OptionPlacement,
    ) -> Option<Vec<usize>> {
        let count = arguments.exact_argv_len()?;
        let end = count.saturating_sub(self.reserved_word_count(arguments)?);
        let options = self.available().collect::<Vec<_>>();
        let mut reachable = vec![false; end.checked_add(1)?];
        *reachable.get_mut(self.positional_prefix_words)? = true;
        let mut possible = Vec::new();
        for index in self.positional_prefix_words..end {
            if !reachable[index] {
                continue;
            }
            match arguments.literal_at(index) {
                Some("--") => {}
                Some(word) if !word.starts_with('-') || word == "-" => {
                    if placement == crate::OptionPlacement::Anywhere {
                        reachable[index + 1] = true;
                    }
                }
                Some(word) => {
                    possible.push(index);
                    if let Some(option) = crate::spec::resolve_available_option_prefix_with(
                        &options,
                        word,
                        self.prefix_matching,
                    ) {
                        Self::retain_possible_option_successors(
                            &mut reachable,
                            arguments,
                            index,
                            option,
                        );
                    }
                }
                None => {
                    possible.push(index);
                    for option in options.iter().copied().filter(|option| option.name != "--") {
                        Self::retain_possible_option_successors(
                            &mut reachable,
                            arguments,
                            index,
                            option,
                        );
                    }
                }
            }
        }
        Some(possible)
    }

    fn retain_possible_option_successors(
        reachable: &mut [bool],
        arguments: crate::InvocationArguments<'_>,
        index: usize,
        option: &OptionSpec,
    ) {
        let first = index + 1;
        if let Some(width) = option.value_word_count_for_arguments(arguments, index) {
            if let Some(next) = first
                .checked_add(width)
                .and_then(|next| reachable.get_mut(next))
            {
                *next = true;
            }
        } else {
            // An unresolved authored width can consume any later slot. Keep
            // every remaining position possible rather than guessing a width.
            reachable[first..].fill(true);
        }
    }

    /// Options admitted by the retained ingress surface and package floor.
    /// Shared and form-local rows keep their own inherited surfaces.
    pub fn available(self) -> impl Iterator<Item = &'r OptionSpec> {
        let admits = move |option: &&OptionSpec, parent| {
            option.supports_dialect(self.availability.query, parent)
                && option.available_for_version(self.availability.package_version)
        };
        self.base
            .iter()
            .filter(move |option| admits(option, self.parent_surface))
            .chain(
                self.form
                    .iter()
                    .filter(move |option| admits(option, self.form_surface)),
            )
    }

    /// Whether an option occurs in the proved leading prefix. Unknown prefix
    /// words decline; unknown values of a declared fixed-width option remain
    /// ordinary values and do not change this selection.
    #[must_use]
    pub fn prefix_contains(
        self,
        arguments: crate::InvocationArguments<'_>,
        name: &str,
    ) -> Option<bool> {
        let end = self.leading_word_count(arguments)?;
        let options = self.available().collect::<Vec<_>>();
        let mut found = false;
        let mut index = self.positional_prefix_words;
        while index < end {
            let option = crate::spec::resolve_available_option_prefix_with(
                &options,
                arguments.literal_at(index)?,
                self.prefix_matching,
            )?;
            found |= option.name == name;
            index += 1 + option.value_word_count_for_arguments(arguments, index)?;
        }
        Some(found)
    }

    /// Prove the end of the option prefix after any authored constructor words.
    /// Unknown option words retain uncertainty; fixed option values keep slots.
    #[must_use]
    pub fn leading_word_count(self, arguments: crate::InvocationArguments<'_>) -> Option<usize> {
        let table = self.available().collect::<Vec<_>>();
        if table.is_empty() {
            return Some(0);
        }
        if arguments.exact_argv_len()? < self.positional_prefix_words {
            return None;
        }
        let consumed = crate::spec::leading_option_word_count_for_arguments(
            &table,
            arguments.slice_from(self.positional_prefix_words),
            self.prefix_matching,
            self.reserved_word_count(arguments)?,
        )?;
        self.positional_prefix_words.checked_add(consumed)
    }
}

/// Target-neutral semantic and effect descriptors for an invocation.
///
/// Every field is derived from command-registry data.  In particular this
/// structure has no `TclVM`, `WASM`, `BPF`, or other backend code-generation
/// hook.
/// `lowering_hook` is the existing common front-end structural-lowering
/// descriptor; it is retained so the current compiler can adopt this API
/// without changing behaviour.
#[derive(Debug, Clone, Copy)]
pub struct InvocationSemantics<'r, 'w> {
    script_metadata: crate::selected_script_timing::SelectedScriptMetadata<'r>,
    /// Selected named-object factory metadata. A candidate class does not
    /// establish a live object allocation or method implementation.
    pub named_object_factory: Option<NamedObjectFactory>,
    /// Registry-selected static semantic operation identity.
    ///
    /// This is not a live command identity; runtime command binding and trace
    /// state are established by later common analyses.
    pub operation: SemanticOperationId,
    /// Effective codegen descriptor under the same selected form, member and
    /// availability as this view. This is metadata, not native entry proof.
    pub codegen_hook: Option<CodegenHookId>,
    /// Effective registry-selected analyser semantic hook.
    pub analyser_hook: Option<crate::hooks::AnalyserHookId>,
    /// Effective target-neutral completion contract.
    ///
    /// The resolver selects a matching form first, then a resolved subcommand,
    /// then the command descriptor, falling back to the conservative generic
    /// invoke contract only when the registry supplied none of them.
    pub completion: CompletionDescriptor,
    /// Registry-declared dependency contract for the invocation's result.
    pub result_stability: ResultStability,
    /// Authored native result dependency, requiring actual implementation proof.
    pub native_result: Option<crate::native_result::NativeResultContract>,
    /// Effective Tcl value-representation effect.
    pub representation_effect: RepresentationEffect,
    /// Selected operand representation contracts, before argv layout projection.
    pub arg_types: &'static [(u8, crate::hooks::ArgTypeHint)],
    /// Additive command and subcommand behaviour traits.
    pub traits: Traits,
    /// Selected command's authored getter source colour, before argc selection.
    pub taint_source: Option<crate::taint::TaintColour>,
    /// Selected successful-result transform, before frozen operand validation.
    pub taint_transform: Option<crate::taint::SelectedTaintTransform>,
    /// Whether the selected command/subcommand/form mutates its receiver or
    /// other command-specific state.
    pub mutator: bool,
    /// Effective arity after command/subcommand/form resolution.
    pub arity: Arity,
    /// Number of leading post-head words before `arg_roles` starts.
    ///
    /// It is one for a subcommand form and zero otherwise.
    pub argument_offset: usize,
    /// Effective static argument-role declaration.
    pub arg_roles: &'r [(u8, ArgRole)],
    /// Dynamic argument-role resolver when the effective descriptor uses one.
    ///
    /// A matched form has only static roles and therefore supplies `None`.
    pub arg_role_resolver: Option<ArgRoleResolver>,
    /// Exact-cardinality resolver, independent of operand contents.
    pub arg_role_count_resolver: Option<ArgRoleCountResolver>,
    /// Native structured layout projection, retaining unknown operand values.
    pub arg_role_layout_resolver: Option<ArgRoleLayoutResolver>,
    /// Authored roles that remain possible when computed operands prevent
    /// selecting the resolver's exact positions. This never assigns a role
    /// to a particular argument or establishes a physical mutation.
    pub arg_role_resolver_roles: &'static [ArgRole],
    /// Structural repeating argument roles, independent of operand values.
    pub repeated_args: &'r [crate::RepeatedArgLayout],
    /// The effective shared clause grammar; forms retain their own static roles.
    pub clause_grammar: Option<&'r crate::clause_grammar::ClauseGrammarSpec>,
    /// Effective command/subcommand/form option descriptors.
    pub options: InvocationOptions<'r, 'w>,
    /// Where the options' effects come from — the families, reservation,
    /// prefix policy and inherited release gate of the selected option table
    /// (read by [`ResolvedInvocation::option_effects`]).
    pub option_scope: OptionEffectScope<'r>,
    /// Result Tcl internal-representation type, when declared.
    pub return_type: Option<TclType>,
    /// Authored native procedure definition grammar.
    pub procedure_definition: Option<crate::native_procedure::NativeProcedureDefinitionSpec>,
    /// Selected native compiler descriptor; syntax and compiler entry remain required.
    pub native_compilation: Option<crate::native_compilation::NativeCompilationSpec>,
    /// Successful handler transfer without compiler or opcode proof.
    pub successful_handler: Option<crate::native_compilation::SuccessfulHandlerSpec>,
    /// Selected receiver naming grammar. Argument indices are descriptor-local;
    /// `argument_offset` supplies the effective post-head coordinate. This does
    /// not prove a successful store or a normal read.
    pub variable_receivers: Option<&'static [(u8, VariableReceiverOperandForm)]>,
    /// Selected native byte-array transformation contract.
    pub byte_array_effect: crate::ByteArrayEffect,
    /// Selected payload getter/sink operand layout.
    pub byte_array_payload: Option<crate::BytePayloadSpec>,
    /// Dialects in which this invocation safely initialises an unset target.
    pub safe_on_uninit: Option<&'static [SpecSurface]>,
    /// How the invocation types variables it writes.
    pub var_write_typing: VarWriteTyping,
    /// Result-to-container-element relationship, when declared.
    pub return_elements: Option<ReturnElements>,
    /// In-place container-element evolution, when declared.
    pub var_elements_effect: Option<VarElementsEffect>,
    /// Frame-crossing argument grammar, when declared by the command.
    pub frame_effect: Option<FrameEffectSpec>,
    /// Whether body arguments run as ordinary scripts or structural bodies.
    pub body_kind: BodyKind,
    /// Registry-authored immediate script execution grammar.
    pub body_execution: Option<crate::body_execution::BodyExecutionSpec>,
    /// Which interpreter owns evaluated body arguments.
    pub body_interpreter: BodyInterpreter,
    /// Typed common front-end structural-lowering descriptor.
    ///
    /// This is not a target code-generation hook.  It is the existing
    /// registry-to-common-IR dispatch key and will ultimately become the
    /// semantic operation selected by the registry.
    pub lowering_hook: Option<LoweringHookId>,
    /// Structured side effects.  A non-empty resolved-subcommand declaration
    /// takes precedence over the command-level declaration, matching the
    /// existing side-effect classifier.
    pub side_effects: &'r [SideEffect],
    /// Effective declared mutable-world descriptor.
    ///
    /// Command, resolved-subcommand, and form descriptors in composition
    /// order.  Call [`ResolvedInvocation::effects`] to resolve this
    /// cheap descriptor chain and bridge existing effect metadata into the
    /// owned representation common compiler passes consume.
    pub world_effects: ResolvedWorldEffects,
    /// Effective command-binding and variable-cell transition descriptors.
    ///
    /// This remains borrowed until [`ResolvedInvocation::state_transitions`]
    /// materialises owned facts for a common consumer.
    pub state_transitions: ResolvedStateTransitions,
    /// Required live-interpreter stability proofs for this resolution.
    ///
    /// This descriptor chain is independent of backend guard encodings. An
    /// unstamped command resolves to the conservative Tcl dependency set.
    pub dispatch_dependencies: ResolvedDispatchDependencies,
    /// Registry-selected relationship/content validator for literal arguments.
    pub literal_argument_validator: Option<LiteralArgumentValidator>,
    /// The value-transfer declaration state, resolved with the selected
    /// subcommand and form: the innermost explicit declaration or
    /// abstention, or the specialisation derived from a descriptor stating
    /// the same operation (`docs/design/compiler/value-transfers.md`). The
    /// value axis is a projection of this resolution, never a second
    /// resolver.
    pub value: crate::value_transfer::ResolvedSemantics,
}

impl InvocationSemantics<'_, '_> {
    /// Resolver input contract; it does not imply cardinality is known.
    #[must_use]
    pub fn arg_role_resolver_input(&self) -> ArgRoleResolverInput {
        if [
            self.arg_role_resolver.is_some(),
            self.arg_role_count_resolver.is_some(),
            self.arg_role_layout_resolver.is_some(),
        ]
        .into_iter()
        .filter(|present| *present)
        .count()
            > 1
        {
            ArgRoleResolverInput::ConflictingResolvers
        } else if self.arg_role_layout_resolver.is_some() {
            ArgRoleResolverInput::StructuredLayout
        } else if self.arg_role_count_resolver.is_some() {
            ArgRoleResolverInput::Cardinality
        } else {
            ArgRoleResolverInput::LiteralValues
        }
    }
}

/// A command invocation resolved to target-neutral registry semantics.
///
/// The structure retains caller word facts while exposing canonical names and
/// a backend-independent semantic projection. It neither assumes
/// that the command's runtime binding is immutable nor proves a native
/// specialisation is safe; later command-environment and trace analyses supply
/// those dynamic proofs.
#[derive(Debug, Clone, Copy)]
pub struct ResolvedInvocation<'r, 'w> {
    /// Original command-head and post-head source-word facts.
    pub words: InvocationWords<'w>,
    /// Registry canonical command name.
    pub canonical_command: &'static str,
    /// Typed subcommand-resolution outcome, preserving the source spelling.
    pub subcommand: SubcommandResolution<'w>,
    /// Matched command or subcommand form, projected without backend hooks.
    pub form: Option<ResolvedForm<'r>>,
    /// Effective target-neutral semantic and effect descriptors.
    pub semantics: InvocationSemantics<'r, 'w>,
    /// The surface query the invocation was resolved under — `None` for a
    /// dialect-blind resolution. Every derived query answers under it.
    pub dialect: Option<SurfaceQuery<'w>>,
    /// The descriptors the registry selected — the derived queries' own
    /// inputs, beside the effective [`Self::semantics`].
    pub(crate) selected: SelectedDescriptors<'r>,
}

/// The command (or class) and subcommand (or instance method) descriptors an
/// invocation resolved to.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SelectedDescriptors<'r> {
    /// The command's descriptor — the class command's for an instance call.
    pub(crate) spec: &'r CommandSpec,
    /// The resolved subcommand, or the instance method.
    pub(crate) sub: Option<&'r SubCommand>,
    /// Whether `sub` is an instance method reached through an object, whose
    /// class command's own tables do not apply.
    pub(crate) instance: bool,
}

/// Count under an authored argument axis, retaining effective operand indices.
/// Unknown expansion contributes no guaranteed entries; this source layout
/// establishes neither a substituted argv value nor successful invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvocationArgumentCount {
    /// Guaranteed contributing operands under the selected count axis.
    pub minimum: u16,
    /// At least one source operand has unknown argv cardinality.
    pub indeterminate: bool,
    /// Effective post-head indices of the guaranteed counted operands.
    pub operands: Vec<usize>,
}

/// Selected original-source signature with its package-version windows.
/// This descriptive shape retains no native handler or completion authority.
#[derive(Debug, Clone)]
pub struct AuthoredSourceArity<'r> {
    /// Already selected owning descriptor, for its actual version axis.
    pub command: &'r CommandSpec,
    /// Selected member when the argument offset consumes a selector.
    pub subcommand: Option<&'r SubCommand>,
    /// Effective fallback signature, including form-specific overrides.
    pub arity: Arity,
    /// Declared version-dependent shapes on the owning package axis.
    pub windows: &'static [crate::arity::ArityWindow],
    /// A bare ensemble whose selected signature may require a selector.
    pub missing_subcommand: bool,
    /// Selected descriptive usage string for reporting.
    pub synopsis: Option<&'static str>,
    /// Count and original effective operand positions for the fallback shape.
    pub count: InvocationArgumentCount,
}

/// Lambda-literal source position and guaranteed trailing argument geometry.
/// This is descriptive call syntax, not an entered lambda or parameter binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoredSourceLambdaCall {
    /// Effective operand containing the selected lambda-list contract.
    pub lambda_argument: usize,
    /// Guaranteed trailing arguments, with expansion uncertainty retained.
    pub count: InvocationArgumentCount,
}

/// Expression positions in the effective source argv and the selected
/// concatenation grammar. These are readonly source roles, not runtime values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoredSourceExpressionArguments {
    /// Effective post-head operand ordinals.
    pub arguments: Vec<usize>,
    /// The selected grammar concatenates its entire post-head argument tail.
    pub concatenates: bool,
}

/// Value ordinals of the selected append source contract. The variable and
/// payloads remain separate from written geometry and runtime cell values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoredSourceAppendArguments {
    /// Effective operand that names the target variable.
    pub variable: usize,
    /// Effective trailing payload ordinals with exact source cardinality.
    pub values: std::ops::Range<usize>,
}

/// One case action operand selected for readonly quoting advice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthoredSourceCaseBody {
    /// Effective post-head operand ordinal.
    pub argument: usize,
    /// Whether the actual selected case grammar uses regular expressions.
    pub regexp: bool,
    /// This action is the single clause-list word rather than an inline body.
    pub single_block: bool,
}

/// Effective authored procedure declaration positions. This layout accepts
/// no native parameter grammar and proves no publication or entered body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthoredSourceProcedureArguments {
    /// Original effective post-head procedure-name ordinal.
    pub name: usize,
    /// Original effective post-head formal-list ordinal.
    pub parameters: usize,
    /// Original effective post-head deferred-body ordinal.
    pub body: usize,
}

/// Registry descriptors already selected for readonly source assistance.
/// These references retain their selected availability and argv context;
/// they cannot establish installed handlers, execution or rewrite permission.
#[derive(Debug, Clone, Copy)]
pub struct AuthoredSourceDescriptors<'r> {
    /// The selected root command descriptor.
    pub command: &'r crate::CommandSpec,
    /// The selected subcommand descriptor, absent for unresolved selectors.
    pub subcommand: Option<&'r crate::SubCommand>,
}

/// Conditional authored publication layout, without command creation or lifetime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthoredSourceCommandPublicationKind {
    /// A descriptor names a future command, independently of its implementation.
    Command,
    /// A mandatory named factory describes a possible instance class.
    Instance {
        /// Descriptor's conditional instance class, without native allocation.
        class_name: &'static str,
    },
}

/// Effective naming operand selected by the same exact source invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthoredSourceCommandPublication {
    /// Effective post-head naming position, including any selected member.
    pub argument: usize,
    /// Source-only publication purpose; no successful creation follows.
    pub kind: AuthoredSourceCommandPublicationKind,
}

/// Selector diagnosis from the same selected command and availability context.
/// This readonly result grants neither entered dispatch nor rewrite authority.
#[derive(Debug, Clone)]
pub enum AuthoredSourceSubcommandDiagnostic {
    /// A literal selector absent from the admitted table and all declared rows.
    Unknown {
        /// Canonical admitted spellings for suggestions.
        candidates: Vec<&'static str>,
    },
    /// A literal selector abbreviates several admitted rows.
    Ambiguous {
        /// Matching canonical spellings from the common keyword owner.
        candidates: Vec<&'static str>,
    },
    /// A declared row is excluded by the actual context.
    Disabled {
        /// Canonical spelling from the unfiltered descriptor table.
        canonical: &'static str,
        /// Explicit or inherited availability of that row.
        surface: Option<&'static [SpecSurface]>,
    },
}

/// One declared option selected for readonly source diagnostics.
/// Availability remains separate so excluded exact spellings can be explained.
#[derive(Debug, Clone)]
pub struct AuthoredSourceOption<'r> {
    /// Effective post-head ordinal, including captured selector operands.
    pub argument: usize,
    /// Descriptor selected by the shared exact/prefix vocabulary.
    pub option: &'r OptionSpec,
    /// Actual retained surface and package floor admit this descriptor.
    pub available: bool,
    /// Inherited surface for lifecycle and declared-target diagnostics.
    pub surface: Option<&'static [SpecSurface]>,
    /// Effective ordinals of present value words; unknown width or missing
    /// values retain `None` and stop further option interpretation.
    pub values: Option<std::ops::Range<usize>>,
}

/// The first boundary where a selected source option scan stops.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthoredSourceOptionBoundary {
    /// An ordinary data word, whose value can remain dynamic.
    Positional(usize),
    /// The next data-or-option word has no original literal value.
    Dynamic(usize),
    /// An explicitly written or captured terminator.
    Terminator(usize),
    /// A literal option spelling outside the selected vocabulary.
    Unknown(usize),
    /// An ambiguous spelling in the actual admitted option vocabulary.
    Ambiguous {
        /// Effective post-head ordinal of the ambiguous spelling.
        argument: usize,
        /// Canonical candidates from the shared keyword resolver.
        candidates: Vec<&'static str>,
    },
    /// No data operand follows the known option prefix.
    End,
    /// Dynamic selection or value width prevents further interpretation.
    Indeterminate,
}

/// Source option topology from one selected descriptor and actual argv.
/// This supplies neither handler acceptance nor data value or edit authority.
#[derive(Debug, Clone)]
pub struct AuthoredSourceOptionScan<'r> {
    /// Known prefix observations before any unresolved boundary.
    pub options: Vec<AuthoredSourceOption<'r>>,
    /// The selected vocabulary declares an admitted `--` terminator.
    pub accepts_terminator: bool,
    /// Already selected canonical selector path used in reporting.
    pub subcommands: Vec<&'static str>,
    /// Exact reason and coordinate where option interpretation stops.
    pub boundary: AuthoredSourceOptionBoundary,
}

/// Possible option operands from one selected source descriptor and actual
/// argument layout. These ordinals grant no evaluated value, accepted handler,
/// successful completion or editable source correspondence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoredSourceOptionArguments {
    /// Effective post-head ordinals that can be parsed as options.
    pub arguments: Vec<usize>,
    /// Already selected canonical selector path, for presentation only.
    pub subcommands: Vec<&'static str>,
}

/// Relationship grammar and source facts from the same selected option scan.
/// Effective ordinals are distinct from original written anchors. Incomplete
/// source facts cannot prove missing options or actual runtime values.
#[derive(Debug, Clone)]
pub struct AuthoredSourceOptionRelationships<'r> {
    /// Available option topology under the unchanged full context.
    pub scan: AuthoredSourceOptionScan<'r>,
    /// Effective ordinals classified as positional data by this grammar.
    pub positionals: Vec<usize>,
    /// All source positions and values were statically classified.
    pub complete: bool,
    /// Selected descriptor and matching-form relation metadata.
    pub relations: Vec<&'static crate::OptionRelation>,
    /// Optional authored constraints callback, with no execution authority.
    pub constraints: Option<crate::ConstraintsHook>,
}

/// Authored class candidate and name operand of a selected naming factory.
/// This inert metadata cannot prove a created command, successful invocation,
/// source declaration or object-method dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NamedObjectFactory {
    argument: u8,
    class_name: &'static str,
}

impl NamedObjectFactory {
    /// Post-head argument naming the object command under this contract.
    #[must_use]
    pub fn argument(self) -> usize {
        usize::from(self.argument)
    }

    /// Authored nominal class candidate, independently of runtime identity.
    #[must_use]
    pub const fn class_name(self) -> &'static str {
        self.class_name
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SelectedArgumentTypeHints {
    table: &'static [(u8, crate::hooks::ArgTypeHint)],
    first_positional: usize,
}

impl SelectedArgumentTypeHints {
    fn at(self, index: usize) -> Option<&'static crate::hooks::ArgTypeHint> {
        let relative = u8::try_from(index.checked_sub(self.first_positional)?).ok()?;
        self.table
            .iter()
            .find(|(position, _)| *position == relative)
            .map(|(_, hint)| hint)
    }
}

/// An owned, target-neutral projection of a resolved registry invocation.
///
/// This is the hand-off shape for executable IR and shared optimisation
/// passes.  It contains facts selected by the registry, not a claim that the
/// command's runtime binding, namespace lookup, aliases, traces, or `unknown`
/// handling have been proven stable.  It deliberately excludes every backend
/// and lowering hook.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvocationFacts {
    deferred_script_arguments: Option<Vec<usize>>,
    script_lookup_arguments: Option<Vec<(usize, crate::ScriptLookupScope)>>,
    command_prefix_arguments: Option<Vec<(usize, crate::AppendedArity)>>,
    argument_type_hints: Option<SelectedArgumentTypeHints>,
    /// Selected normal naming-factory contract, independent of object identity.
    pub named_object_factory: Option<NamedObjectFactory>,
    /// Canonical registry command identity used to select these facts.
    pub canonical_command: String,
    /// The complete owned subcommand-resolution outcome.
    pub subcommand: OwnedSubcommandResolution,
    /// Registry form identity, when a form was selected from a determinate
    /// argv shape.
    pub form: Option<String>,
    /// Target-neutral semantic operation selected by the registry.
    pub operation: SemanticOperationId,
    /// Selected logical structural hook, independently of native admission.
    pub lowering_hook: Option<LoweringHookId>,
    /// Effective registry-selected analyser semantic hook.
    pub analyser_hook: Option<crate::hooks::AnalyserHookId>,
    /// Effective Tcl completion descriptor.
    pub completion: CompletionDescriptor,
    /// Registry-declared dependency contract for the invocation's result.
    pub result_stability: ResultStability,
    /// Authored native result dependency, requiring actual implementation proof.
    pub native_result: Option<crate::native_result::NativeResultContract>,
    /// Effective Tcl value-representation effect.
    pub representation_effect: RepresentationEffect,
    /// Native coercion phase projected from the selected form and actual argv.
    /// This can affect shared objects without writing their variable cells.
    pub operand_representation_coercions: crate::representation::RepresentationCoercionSelection,
    /// Fully resolved mutable-world footprint.
    ///
    /// Writes covered by [`Self::transition_effect_coverage`] have already
    /// been removed here, while independent reads, clobbers, callbacks, and
    /// other domains remain present.
    pub effects: EffectFootprint,
    /// Complete registry transition facts, or a typed wildcard obligation for
    /// an unstamped generic Tcl invocation.
    pub state_transitions: StateTransitionKnowledge,
    /// Registry contract identifying legacy or explicit writes whose
    /// completion-edge transition facts are authoritative.
    pub transition_effect_coverage: TransitionEffectCoverages,
    /// Mutable Tcl domains that must be proven stable before specialising this
    /// statically resolved invocation.
    pub dispatch_dependencies: DispatchDependencies,
    /// Effective command and subcommand traits.
    pub traits: Traits,
    /// Authored successful result source colour. Actual implementation proof
    /// remains required; this is not catalogue-based source classification.
    pub taint_source: Option<crate::taint::TaintColour>,
    /// Successful-result transform validated against actual frozen operands.
    /// Handler provenance remains an independent consumer requirement.
    pub taint_transform: Option<crate::taint::TaintColour>,
    /// Whether the selected invocation form mutates command-specific state.
    pub mutator: bool,
    /// Effective arity after command, subcommand, and form selection.
    pub arity: Arity,
    /// Count under the authored option/positional grammar of the frozen argv.
    /// Unknown option layout retains `None`, rather than a guessed argc.
    pub arity_argument_count: Option<u16>,
    /// Actual frozen post-head argv cardinality, independent of count axis.
    pub frozen_argument_count: Option<usize>,
    /// Number of leading post-head words before static argument roles start.
    pub argument_offset: usize,
    /// Effective argument-role declarations for this invocation.
    ///
    /// A registry resolver is evaluated here when every argument is literal.
    /// Otherwise this retains the descriptor's conservative static roles and
    /// [`Self::arg_roles_complete`] is `false`.
    pub arg_roles: Vec<(u8, ArgRole)>,
    /// Repeated layouts retaining conditional-binding policy for consumers.
    pub repeated_args: Vec<crate::RepeatedArgLayout>,
    /// Whether [`Self::arg_roles`] is the complete role assignment.
    ///
    /// `false` means registry metadata has an argument-role resolver whose
    /// result depends on literal invocation arguments. Consumers must retain
    /// the dynamic role obligation rather than treating this static slice as a
    /// complete answer.
    pub arg_roles_complete: bool,
    /// Closed capability inventory for unresolved value-dependent role positions.
    /// Meaningful as uncertainty only while `arg_roles_complete` is false.
    pub arg_role_resolver_roles: &'static [ArgRole],
    /// Declared result internal-representation type, when available.
    pub return_type: Option<TclType>,
    /// Native procedure argv selection, preserving invalid/unknown outcomes.
    pub procedure_definition: Option<crate::native_procedure::NativeProcedureDefinitionSelection>,
    /// Native compilation capability independent of runtime target selection.
    pub native_compilation: Option<crate::native_compilation::NativeCompilationSpec>,
    /// Successful handler transfer without compiler or opcode proof.
    pub successful_handler: Option<crate::native_compilation::SuccessfulHandlerSpec>,
    /// Selected receiver naming grammar. Argument indices are descriptor-local;
    /// `argument_offset` supplies the effective post-head coordinate. This does
    /// not prove a successful store or a normal read.
    pub variable_receivers: Option<&'static [(u8, VariableReceiverOperandForm)]>,
    /// Selected native byte-array transformation contract.
    pub byte_array_effect: crate::ByteArrayEffect,
    /// Selected payload getter/sink operand layout.
    pub byte_array_payload: Option<crate::BytePayloadSpec>,
    /// How written variables receive types from this invocation.
    pub var_write_typing: VarWriteTyping,
    /// Result-to-container-element relationship, when declared.
    pub return_elements: Option<ReturnElements>,
    /// In-place container-element evolution, when declared.
    pub var_elements_effect: Option<VarElementsEffect>,
    /// Whether body words run in the caller frame or a structural scope.
    pub body_kind: BodyKind,
    /// Registry-authored immediate script execution grammar.
    pub body_execution: Option<crate::body_execution::BodyExecutionSpec>,
    /// Which interpreter owns evaluated body arguments.
    pub body_interpreter: BodyInterpreter,
    /// Frame-crossing argument grammar, when declared.
    pub frame_effect: Option<FrameEffectSpec>,
}

/// Selected runtime variable-name receiver grammar, independently of lexical
/// roots, compiler locals, namespace alias declarations and successful access.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariableReceiverOperandForm {
    /// A single argv object may contain a combined root and element spelling.
    Combined,
    /// A trace variable subject uses its independently selected name ingress
    /// before separating a root and element. This grants no trace operation.
    TraceSubject,
}

impl VariableReceiverOperandForm {
    /// Purpose-selected original operand geometry. Registration, observer
    /// lifetime and successful access remain independent of this projection.
    #[must_use]
    pub fn input_form(
        self,
        protocol: tcl_syntax::naming::NativeNameProtocol,
        original: &[u8],
    ) -> Option<tcl_syntax::naming::NativeVariableInputForm<'_>> {
        use tcl_syntax::naming::NativeVariableInputForm;
        let selected = match self {
            Self::Combined => original,
            Self::TraceSubject => protocol
                .trace_query_input(original)
                .ok()?
                .borrowed_selected()?,
        };
        Some(NativeVariableInputForm::Combined(selected))
    }
}

fn selected_variable_receiver_operand_form(
    argument: usize,
    argument_offset: usize,
    roles: &[(u8, ArgRole)],
    traits: Traits,
    receivers: Option<&[(u8, VariableReceiverOperandForm)]>,
    handler: Option<crate::native_compilation::SuccessfulHandlerSpec>,
    compiler: Option<crate::native_compilation::NativeCompilationSpec>,
) -> Option<VariableReceiverOperandForm> {
    use crate::native_compilation::{
        NativeCompilationGrammar as Grammar, SuccessfulHandlerSpec as Handler,
    };
    if traits.contains(Traits::CREATES_SCOPE_ALIAS)
        || !roles.iter().any(|&(index, role)| {
            argument_offset.checked_add(usize::from(index)) == Some(argument)
                && matches!(role, ArgRole::VarRead | ArgRole::VarWrite)
        })
    {
        return None;
    }
    if let Some(receivers) = receivers {
        return receivers.iter().find_map(|&(index, form)| {
            (argument_offset.checked_add(usize::from(index)) == Some(argument)).then_some(form)
        });
    }
    let handler = matches!(
        handler,
        Some(
            Handler::VariableOperands
                | Handler::ConditionalVariableOperands(_)
                | Handler::InitialiseEmptyVariable
                | Handler::CatchOutputs
                | Handler::DictionaryScope
        )
    );
    let compiler = compiler.is_some_and(|spec| {
        matches!(
            spec.grammar,
            Grammar::VariableLoadStore
                | Grammar::Increment
                | Grammar::VariableAppend(_)
                | Grammar::InfoExists
                | Grammar::ListAssignment
                | Grammar::Catch
                | Grammar::Try
        )
    });
    (handler || compiler).then_some(VariableReceiverOperandForm::Combined)
}

impl InvocationFacts {
    /// Original argv receiver form selected by the actual handler descriptor.
    /// A nominal variable role alone cannot select this protocol. Whole-array
    /// operations still parse a combined name; an element may be a guest error.
    #[must_use]
    pub fn variable_receiver_operand_form(
        &self,
        argument: usize,
    ) -> Option<VariableReceiverOperandForm> {
        if !self.arg_roles_complete || self.arity_accepts_frozen_arguments() != Some(true) {
            return None;
        }
        selected_variable_receiver_operand_form(
            argument,
            self.argument_offset,
            &self.arg_roles,
            self.traits,
            self.variable_receivers,
            self.successful_handler,
            self.native_compilation,
        )
    }

    /// Selected deferred single-script/prefix operand positions. Unknown values
    /// keep their slots; None retains unknown layout, timing or concatenation.
    /// This metadata supplies positive navigation only, never callback absence,
    /// execution, future body admission, edit completeness or erasure permission.
    #[must_use]
    pub fn deferred_script_argument_indices(&self) -> Option<&[usize]> {
        self.deferred_script_arguments.as_deref()
    }

    /// Lookup-frame purpose of one selected executable argument. The index
    /// addresses effective post-head argv, including ensemble selectors.
    /// No receiving-head site, future table, callback reach or normal entry is
    /// implied. Unknown positions and reference-only forms return `None`.
    #[must_use]
    pub fn script_lookup_scope(&self, argument_index: usize) -> Option<crate::ScriptLookupScope> {
        self.script_lookup_arguments
            .as_ref()?
            .iter()
            .find_map(|&(index, scope)| (index == argument_index).then_some(scope))
    }

    /// Appended argument count of one selected executable prefix. The ordinal
    /// addresses effective post-head argv. Reference-only, rejected and
    /// unclassified positions cannot borrow another operand's prefix purpose.
    #[must_use]
    pub fn command_prefix_arity(&self, argument_index: usize) -> Option<crate::AppendedArity> {
        self.command_prefix_arguments
            .as_ref()?
            .iter()
            .find_map(|&(index, arity)| (index == argument_index).then_some(arity))
    }

    /// Positional representation advice retained from this selected invocation.
    /// It establishes neither actual representation nor a conversion result.
    /// Unknown cardinality/options and rejected arity supply no hint.
    #[must_use]
    pub fn argument_type_hint(&self, index: usize) -> Option<&'static crate::hooks::ArgTypeHint> {
        if self.arity_accepts_frozen_arguments() != Some(true)
            || index >= self.frozen_argument_count?
        {
            return None;
        }
        self.argument_type_hints?.at(index)
    }

    /// Actual normal native numeric result shape, independently of operand
    /// contents, callback effects, completion guarantees or compiler selection.
    #[must_use]
    pub fn normal_numeric_result_production(
        &self,
        arguments: crate::InvocationArguments<'_>,
    ) -> Option<crate::native_result::NativeNumericResultProduction> {
        if self.successful_handler != Some(crate::native_compilation::SuccessfulHandlerSpec::Leaf)
            || self.arity_accepts_frozen_arguments() != Some(true)
            || self.frozen_argument_count != arguments.exact_argv_len()
        {
            return None;
        }
        if matches!(
            self.native_result,
            Some(crate::native_result::NativeResultContract::ScalarMath(_))
        ) {
            return self
                .normal_scalar_math_protocol(arguments)
                .map(crate::mathfunc::NativeScalarMathProtocol::result_production);
        }
        if self.operation != SemanticOperationId::Intrinsic(IntrinsicId::ListLength) {
            return None;
        }
        self.native_result?
            .normal_numeric_result_production(arguments, self.argument_offset)
    }

    /// Authored scalar math operand/result protocol from this selected native
    /// handler. Actual reached implementation and observers remain independent.
    #[must_use]
    pub fn normal_scalar_math_protocol(
        &self,
        arguments: crate::InvocationArguments<'_>,
    ) -> Option<crate::mathfunc::NativeScalarMathProtocol> {
        if self.operation != SemanticOperationId::Invoke
            || self.successful_handler
                != Some(crate::native_compilation::SuccessfulHandlerSpec::Leaf)
            || self.arity_accepts_frozen_arguments() != Some(true)
            || self.frozen_argument_count != arguments.exact_argv_len()
        {
            return None;
        }
        let crate::native_result::NativeResultContract::ScalarMath(operation) =
            self.native_result?
        else {
            return None;
        };
        crate::mathfunc::NativeScalarMathProtocol::for_invocation(
            operation,
            arguments,
            self.argument_offset,
        )
    }

    /// Effect-purpose refinement after proving (or failing to prove) the
    /// original scalar operand's string-access closure. This changes neither
    /// native result/completion nor compiler/implementation identity.
    pub fn refine_scalar_math_input_effects(
        &mut self,
        arguments: crate::InvocationArguments<'_>,
        input_is_closed: bool,
    ) {
        if self.normal_scalar_math_protocol(arguments).is_some() && !input_is_closed {
            self.effects
                .add_callback(crate::world_effect::CallbackEffect {
                    kinds: crate::world_effect::CallbackKinds::HOST,
                    reentrancy: crate::world_effect::Reentrancy::AnyInterpreter,
                });
            self.traits
                .remove(Traits::PURE | Traits::PURE_EVALUATION | Traits::FRAMELESS_RUNTIME);
        }
    }

    /// Selected normal-result provider after separate proof of the actual
    /// native handler. Operand effects and compiler admission stay independent.
    #[must_use]
    pub fn normal_list_method_provider(
        &self,
        arguments: crate::InvocationArguments<'_>,
    ) -> Option<crate::native_result::NativeListMethodProvider> {
        if self.arity_accepts_frozen_arguments() != Some(true)
            || self.frozen_argument_count != arguments.exact_argv_len()
        {
            return None;
        }
        self.native_result?
            .normal_list_method_provider(arguments, self.argument_offset)
    }

    /// Bound an authored list-length handler after independent proof that its
    /// original operand is an ordinary List/Dictionary object. This descriptor
    /// query requires actual handler identity separately and never proves OK.
    #[must_use]
    pub fn ordinary_list_length_completion(
        &self,
        arguments: crate::InvocationArguments<'_>,
        proved_argument: usize,
        representation: crate::representation::OrdinaryContainerRepresentation,
    ) -> Option<crate::completion_route::InvocationCompletionRoute> {
        if self.operation != SemanticOperationId::Intrinsic(IntrinsicId::ListLength)
            || self.successful_handler
                != Some(crate::native_compilation::SuccessfulHandlerSpec::Leaf)
            || self.arity_accepts_frozen_arguments() != Some(true)
            || self.frozen_argument_count != arguments.exact_argv_len()
        {
            return None;
        }
        self.representation_effect.ordinary_list_length_completion(
            arguments,
            self.argument_offset,
            proved_argument,
            representation,
        )
    }

    /// Select the native list-length object-method effect obligation. The
    /// caller must separately prove the current handler and original operand.
    /// Tcl 9's `lengthProc` returns a size, but arbitrary registered native
    /// methods can mutate the interpreter world; this is not a Tcl code bound.
    #[must_use]
    pub fn list_length_object_protocol(
        &self,
        arguments: crate::InvocationArguments<'_>,
    ) -> Option<crate::representation::ListLengthObjectProtocol> {
        use crate::representation::ListLengthObjectProtocol as Protocol;
        use tcl_dialect::{
            TclVersion,
            model::{Family, Release},
        };
        if self.operation != SemanticOperationId::Intrinsic(IntrinsicId::ListLength)
            || self.successful_handler
                != Some(crate::native_compilation::SuccessfulHandlerSpec::Leaf)
            || self.arity_accepts_frozen_arguments() != Some(true)
            || self.frozen_argument_count != arguments.exact_argv_len()
            || self.argument_offset.checked_add(1) != arguments.exact_argv_len()
        {
            return None;
        }
        let argument = self.argument_offset;
        let Some(dialect) = arguments.dialect() else {
            return Some(Protocol::Unknown);
        };
        Some(match (dialect.family(), dialect.tcl_version) {
            (Some(Family::Tcl), Some(version)) if version >= TclVersion::V9_0 => {
                Protocol::AbstractLength { argument }
            }
            (Some(Family::Tcl), Some(_)) => Protocol::Ordinary { argument },
            (Some(Family::Jim), _)
                if dialect
                    .core_point
                    .is_some_and(|point| point.release() == Release::JIM_0_84) =>
            {
                Protocol::Ordinary { argument }
            }
            _ => Protocol::Unknown,
        })
    }

    /// Signature validity of the argv which produced these owned facts.
    /// This does not validate option values or execution effects.
    #[must_use]
    pub fn arity_accepts_frozen_arguments(&self) -> Option<bool> {
        self.arity_argument_count
            .map(|count| self.arity.accepts(count))
    }

    /// Return the effect footprint ready for completion-aware world-state
    /// projection.
    ///
    /// This deliberately exposes no command spelling or legacy descriptor:
    /// [`Self::effects`] was filtered only through the registry-owned
    /// [`Self::transition_effect_coverage`] contract when these facts were
    /// materialised.
    #[must_use]
    pub const fn world_state_effects(&self) -> &EffectFootprint {
        &self.effects
    }

    /// Return the sole invocation argument carrying one of `roles` when the
    /// resolved command or subcommand accepts the supplied outer arity.
    ///
    /// `argument_count` and the returned index both count words after the
    /// command head, including a subcommand word when one was resolved. This
    /// keeps runtime consumers on the registry's argument-offset and
    /// resolver-first role contract instead of maintaining command-local
    /// operand tables. Multiple requested roles on the same argument still
    /// identify one operand; matching roles on different arguments are
    /// ambiguous. An incomplete dynamic role resolution, invalid arity, or
    /// missing or ambiguous role returns `None`.
    #[must_use]
    pub fn sole_argument_index_for_roles(
        &self,
        argument_count: usize,
        roles: &[ArgRole],
    ) -> Option<usize> {
        if self.frozen_argument_count != Some(argument_count)
            || self.arity_accepts_frozen_arguments() != Some(true)
            || !self.arg_roles_complete
        {
            return None;
        }

        let mut sole = None;
        for &(index, role) in &self.arg_roles {
            if !roles.contains(&role) {
                continue;
            }
            let index = self.argument_offset + usize::from(index);
            if index >= argument_count {
                return None;
            }
            match sole {
                None => sole = Some(index),
                Some(previous) if previous == index => {}
                Some(_) => return None,
            }
        }
        sole
    }
}

/// Retain the selected option tables and their availability context.
pub(crate) fn invocation_options<'r, 'w>(
    spec: &'r CommandSpec,
    sub: Option<&'r SubCommand>,
    form: Option<&'r CommandForm>,
    availability: InvocationAvailability<'w>,
) -> InvocationOptions<'r, 'w> {
    let parent_surface = sub.and_then(|sub| sub.surface).or(spec.surface);
    InvocationOptions {
        availability,
        parent_surface,
        form_surface: form.and_then(|form| form.surface).or(parent_surface),
        prefix_matching: sub.map_or(spec.prefix_matching, |sub| sub.prefix_matching),
        positional_prefix_words: if let Some(sub) = sub {
            sub.option_prefix_words
        } else if spec.option_prefix_words > 0 {
            spec.option_prefix_words
        } else {
            spec.constructor_prefix_words()
                .unwrap_or(match spec.body_execution {
                    Some(crate::body_execution::BodyExecutionSpec::CapturedLifecycle(
                        lifecycle,
                    )) => lifecycle.leading_arguments,
                    _ => 0,
                })
        },
        reserved_trailing_words: sub.map_or(spec.reserved_trailing_words, |sub| {
            sub.reserved_trailing_words
        }),
        case_list: if sub.is_none() { spec.case_list } else { None },
        base: sub.map_or(spec.options, |sub| sub.options),
        form: form.map_or(&[], |form| form.options),
    }
}

fn selected_nested_options<'r, 'w>(
    mut options: InvocationOptions<'r, 'w>,
    spec: &'r CommandSpec,
    sub: Option<&'r SubCommand>,
    words: InvocationWords<'_>,
) -> InvocationOptions<'r, 'w> {
    let Some(sub) = sub.filter(|sub| !sub.sub_subcommands.is_empty()) else {
        return options;
    };
    let Some(word) = words.arguments().literal_at(1) else {
        return options;
    };
    let scope = sub.option_scope(
        Some(word),
        options.availability.query,
        options.availability.package_version,
        spec.surface,
    );
    if scope.sub_subcommand.is_some() {
        options.base = scope.options;
        options.parent_surface = scope.surface;
        options.positional_prefix_words = scope.option_prefix_words;
    }
    options
}

/// Count the effective argv axis without inventing an option grammar. Unknown
/// expansion width retains the exact lower bound and uncertainty; checked
/// conversion refuses a count wider than the shared arity representation.
#[must_use]
pub fn count_invocation_argv(
    arguments: crate::InvocationArguments<'_>,
    offset: usize,
) -> Option<InvocationArgumentCount> {
    arguments.len().checked_sub(offset)?;
    let tail = arguments.slice_from(offset);
    let indices = (0..tail.len())
        .filter(|&index| {
            tail.get(index)
                .is_some_and(crate::InvocationWord::has_exactly_one_argv_entry)
        })
        .collect::<Vec<_>>();
    Some(InvocationArgumentCount {
        minimum: u16::try_from(indices.len()).ok()?,
        indeterminate: indices.len() != tail.len(),
        operands: indices
            .into_iter()
            .map(|index| offset.checked_add(index))
            .collect::<Option<Vec<_>>>()?,
    })
}

/// Assess the same authored count axis and selected leading-option grammar
/// while retaining effective operand indices. Mandatory pre-option data keeps
/// its count even when a call stops before the full prefix is supplied.
#[must_use]
pub fn count_invocation_argument_layout(
    arity: Arity,
    arguments: crate::InvocationArguments<'_>,
    offset: usize,
    options: InvocationOptions<'_, '_>,
) -> Option<InvocationArgumentCount> {
    if arity.count == crate::arity::ArityCount::Arguments {
        return count_invocation_argv(arguments, offset);
    }
    arguments.len().checked_sub(offset)?;
    let arguments = arguments.slice_from(offset);
    let count = arguments.len();
    // An expanded option vector can consume following words as values.
    arguments.exact_argv_len()?;
    let prefix = options.positional_prefix_words.min(count);
    let end = if count <= options.positional_prefix_words {
        count
    } else {
        options.leading_word_count(arguments)?.max(prefix)
    };
    let indices = (0..prefix).chain(end..count).collect::<Vec<_>>();
    let indeterminate = false;
    let minimum = u16::try_from(indices.len()).ok()?;
    let operands = indices
        .into_iter()
        .map(|index| offset.checked_add(index))
        .collect::<Option<Vec<_>>>()?;
    Some(InvocationArgumentCount {
        minimum,
        indeterminate,
        operands,
    })
}

/// Assess one invocation using the authored count axis and shared option walk.
/// This count does not validate option values, effects or native compilation.
#[must_use]
pub fn count_invocation_arguments(
    arity: Arity,
    arguments: crate::InvocationArguments<'_>,
    offset: usize,
    options: InvocationOptions<'_, '_>,
) -> Option<u16> {
    let count = count_invocation_argument_layout(arity, arguments, offset, options)?;
    (!count.indeterminate).then_some(count.minimum)
}

impl<'r, 'w> ResolvedInvocation<'r, 'w> {
    /// Count the actual frozen argv under its selected signature grammar.
    #[must_use]
    pub fn argument_count_for_arity(&self) -> Option<u16> {
        count_invocation_arguments(
            self.semantics.arity,
            self.words.arguments(),
            self.semantics.argument_offset,
            self.semantics.options,
        )
    }

    pub(crate) fn new(
        words: InvocationWords<'w>,
        spec: &'r CommandSpec,
        sub: Option<&'r SubCommand>,
        form: Option<&'r CommandForm>,
        subcommand: SubcommandResolution<'w>,
        availability: InvocationAvailability<'w>,
    ) -> Self {
        let mut semantics = resolve_invocation_semantics(
            spec,
            sub,
            form,
            true,
            words.arguments().dialect(),
            availability.query.as_ref(),
        );
        semantics.options = invocation_options(spec, sub, form, availability);
        semantics.options = selected_nested_options(semantics.options, spec, sub, words);
        if let Some(nested) = sub.and_then(|sub| {
            sub.nested_native_compilation(words.arguments().slice_from(semantics.argument_offset))
        }) {
            semantics.native_compilation = Some(nested);
        }
        if sub.is_none()
            && form.is_none_or(|form| form.return_type.is_none())
            && let Some(hook) = spec.return_type_hook
        {
            semantics.return_type =
                crate::return_type::resolve_arguments(hook, words.arguments(), semantics.options);
        }
        let dialect = availability.query;
        Self {
            words,
            canonical_command: spec.name,
            subcommand,
            form: form.map(|form| ResolvedForm {
                name: form.name,
                arity: form.arity,
                arg_roles: form.arg_roles,
                options: form.options,
            }),
            semantics,
            dialect,
            selected: SelectedDescriptors {
                spec,
                sub,
                instance: false,
            },
        }
    }

    pub(crate) fn new_instance(
        words: InvocationWords<'w>,
        class_spec: &'r CommandSpec,
        method: &'r SubCommand,
        form: Option<&'r CommandForm>,
        subcommand: SubcommandResolution<'w>,
        availability: InvocationAvailability<'w>,
    ) -> Self {
        let mut semantics = resolve_invocation_semantics(
            class_spec,
            Some(method),
            form,
            false,
            words.arguments().dialect(),
            availability.query.as_ref(),
        );
        semantics.options = invocation_options(class_spec, Some(method), form, availability);
        semantics.options =
            selected_nested_options(semantics.options, class_spec, Some(method), words);
        // A selected setter may configure the owning instance's option table.
        // Explicit method tables keep precedence; query forms carry no setter
        // trait and therefore cannot inherit executable constructor options.
        if semantics
            .traits
            .contains(Traits::CONFIGURES_INSTANCE_OPTIONS)
            && semantics.options.base.is_empty()
        {
            semantics.options.base = class_spec.options;
        }
        let dialect = availability.query;
        Self {
            words,
            canonical_command: class_spec.name,
            subcommand,
            form: form.map(|form| ResolvedForm {
                name: form.name,
                arity: form.arity,
                arg_roles: form.arg_roles,
                options: form.options,
            }),
            semantics,
            dialect,
            selected: SelectedDescriptors {
                spec: class_spec,
                sub: Some(method),
                instance: true,
            },
        }
    }

    /// Resolve the complete mutable-world footprint for this invocation — the
    /// derived-query layer's `effects` answer.
    ///
    /// This is the sole common-compiler entry point for command-level world
    /// effects.  It applies a static descriptor, then any argument-dependent
    /// resolver, then bridges the registry's established command-table,
    /// frame-crossing, and structured side-effect declarations.  Consumers do
    /// not need to inspect command names or raw spec fields.  An invocation
    /// with no explicit world-effect descriptor stays conservatively unknown;
    /// legacy facts are added to that unknown footprint rather than being
    /// mistaken for a proof that no other Tcl-world effect can occur.
    #[must_use]
    pub fn effects(&self) -> EffectFootprint {
        let (transitions, coverage) = self
            .semantics
            .state_transitions
            .resolve_with_effect_coverage(self.words.arguments());
        self.effect_footprint_with_transition_coverage(
            transitions.touches_command_bindings(),
            &coverage,
        )
    }

    fn effect_footprint_with_transition_coverage(
        &self,
        command_table_mutation: bool,
        coverage: &TransitionEffectCoverages,
    ) -> EffectFootprint {
        let mut footprint = if self.semantics.world_effects.is_declared() {
            self.semantics
                .world_effects
                .resolve_with_transition_coverage(self.words.arguments(), coverage)
        } else {
            EffectFootprint::conservative_unknown_invocation()
        };
        // Whether this call mutated the command table is read from the one
        // transition vocabulary (ledger C8), never from a second coarse
        // effect word stamped beside it.
        footprint.extend(EffectFootprint::from_legacy_with_transition_coverage(
            command_table_mutation,
            self.semantics.frame_effect,
            self.semantics.side_effects,
            coverage,
        ));
        footprint
    }

    /// Resolve the command-binding and variable-cell transitions for this
    /// invocation.
    ///
    /// Dynamic, expanded, and opaque operands are represented by typed
    /// unknown subjects and conservative domain widenings; no source spelling
    /// is treated as a runtime Tcl name.
    #[must_use]
    pub fn state_transitions(&self) -> StateTransitions {
        self.semantics
            .state_transitions
            .resolve_with_effect_coverage(self.words.arguments())
            .0
    }

    /// Selected ordinary result-only return metadata preserves lexical
    /// binding names in the separate Logical authoring model. Zero or one
    /// exact result operand has no option fields. This source purpose does
    /// not close Native error-option publication, value callbacks, observers
    /// or normal completion; callers must retain their full Logical input and
    /// prove all original binding-name uses plus isolated editing policy.
    #[must_use]
    pub fn authored_source_result_preserves_variable_bindings(&self) -> bool {
        // naming.minifier.logical-formal-binding-alpha
        // docs/design/analysis/name-resolution-proofs/logical-formal-binding-alpha.md
        let (roles, complete) = self.authored_source_argument_roles();
        let barriers = crate::FRAME_REACH_TRAITS
            | Traits::INTROSPECTS_BY_NAME
            | Traits::TARGETS_VARIABLE_BY_NAME
            | Traits::REFLECTS_COMMAND_NAMES
            | Traits::CREATES_SCOPE_ALIAS
            | Traits::CREATES_DYNAMIC_BARRIER
            | Traits::DEFERS_BODY;
        self.semantics.operation == SemanticOperationId::StructuredLowering(LoweringHookId::Return)
            && self.semantics.native_result
                == Some(crate::native_result::NativeResultContract::ReturnResult)
            && self.semantics.argument_offset == 0
            && self
                .words
                .arguments()
                .exact_argv_len()
                .is_some_and(|count| count <= 1)
            && complete
            && match self.words.arguments().exact_argv_len() {
                Some(0) => roles.is_empty(),
                Some(1) => roles.as_slice() == [(0, ArgRole::Result)],
                _ => false,
            }
            && !self.semantics.traits.intersects(barriers)
            && self.semantics.frame_effect.is_none()
    }

    /// The option-effect answer for this call
    /// (`docs/design/compiler/registry-consumer-contracts.md` § *Options with
    /// semantic effects*): the generic walk over the selected option table —
    /// the subcommand's own, when one was resolved — with the options
    /// available at the invocation's [`Self::dialect`].
    /// [`OptionEffects::option_end`] is a post-head argument index, like
    /// every other index this resolution answers.
    #[must_use]
    pub fn option_effects(&self) -> OptionEffects {
        let dialect = self.dialect;
        let scope = self.semantics.option_scope;
        let options: Vec<&OptionSpec> = self
            .semantics
            .options
            .base
            .iter()
            .chain(self.semantics.options.form)
            .filter(|option| option.supports_dialect(dialect, scope.parent_surface))
            .collect();
        let offset = self.semantics.argument_offset;
        let mut effects = crate::option_effect::option_effects_over(
            &options,
            scope.families,
            self.words.arguments().slice_from(offset),
            scope.reserved_trailing_words,
            dialect,
            scope.prefix_matching,
        );
        effects.option_end += offset;
        effects
    }

    /// Which substitutions this call performs over its own argument text, or
    /// `None` when the command performs none — the projection of
    /// [`Self::option_effects`] onto
    /// [`crate::substitution::SubstitutionKinds`], with the rule
    /// [`crate::CommandSpec::substitutions_performed`] states: an unreadable
    /// call, or one whose option run stops before the reserved operands,
    /// performs every kind.
    #[must_use]
    pub fn substitutions_performed(&self) -> Option<crate::substitution::SubstitutionKinds> {
        if !self
            .semantics
            .traits
            .contains(Traits::PERFORMS_SUBSTITUTION)
        {
            return None;
        }
        let effects = self.option_effects();
        let reaches_operands = self.words.arguments().exact_argv_len().is_some_and(|len| {
            effects.option_end + self.semantics.option_scope.reserved_trailing_words >= len
        });
        Some(if reaches_operands {
            effects.substitution_kinds()
        } else {
            crate::substitution::SubstitutionKinds::ALL
        })
    }

    /// The call's clause plan: the effective clause grammar walked over the
    /// words' values after the head (and after the subcommand word), reported
    /// in the invocation's post-head coordinates.
    ///
    /// `None` when no grammar applies or it is unavailable at the
    /// invocation's [`Self::dialect`], when a `{*}` expansion makes the word
    /// count unknown, or when a computed word sits where the walk compares a
    /// keyword, a noise word or the fall-through marker — Tcl decides those by
    /// value. A computed word in a positional slot (`if $cond {…}`) is fine.
    #[must_use]
    pub fn clause_plan(&self) -> Option<crate::clause_grammar::ClausePlan> {
        self.clause_walk()?.ok()
    }

    /// The walk behind [`Self::clause_plan`], saying where it abstained: `Err`
    /// names the first computed word standing where the walk compares one,
    /// with the call read with every computed word matching nothing
    /// ([`crate::clause_grammar::ClauseAbstention`]). `None` exactly where
    /// [`Self::clause_plan`] has no grammar to walk or no argv shape.
    #[must_use]
    pub fn clause_walk(
        &self,
    ) -> Option<Result<crate::clause_grammar::ClausePlan, crate::clause_grammar::ClauseAbstention>>
    {
        let arguments = self.words.arguments();
        let offset = self
            .semantics
            .argument_offset
            .min(arguments.exact_argv_len()?);
        Some(
            self.semantics
                .clause_grammar?
                .walk_arguments(
                    arguments.slice_from(offset),
                    self.semantics.repeated_args,
                    self.dialect,
                )?
                .map(|plan| plan.offset_by(offset))
                .map_err(|abstention| crate::clause_grammar::ClauseAbstention {
                    word: abstention.word + offset,
                    inert: abstention.inert.offset_by(offset),
                }),
        )
    }

    /// Every word's literal value, a computed word standing in as an inert
    /// empty placeholder — or `None` when an expansion makes the argv shape
    /// unknown. The spelling projection the registry's position-only readers
    /// take; never read a placeholder as a value.
    fn placeholder_spellings(&self) -> Option<Vec<&'w str>> {
        let arguments = self.words.arguments();
        arguments.has_exact_argv_len().then(|| {
            (0..arguments.len())
                .map(|index| arguments.literal_at(index).unwrap_or(""))
                .collect()
        })
    }

    /// The release the invocation's [`Self::dialect`] names on the Tcl
    /// ladder, for a rule whose grammar is a release's numerals.
    fn tcl_version(&self) -> Option<tcl_dialect::TclVersion> {
        match self.dialect?.core.nearest() {
            Some((tcl_dialect::model::Family::Tcl, Some(release))) => {
                tcl_dialect::TclVersion::from_version_string(release)
            }
            _ => None,
        }
    }

    /// The selected command's options available at [`Self::dialect`].
    fn spec_options(&self) -> Vec<&'static OptionSpec> {
        self.selected.spec.option_specs(self.dialect)
    }

    /// Whether the words prove the layout a resolver-derived role depends on
    /// — no expansion, a literal subcommand word, and an option run whose
    /// every word is literal while an option could stand: the registry's
    /// source-layout proof over this resolution's own selection.
    fn layout_is_proven(&self) -> bool {
        let SelectedDescriptors { spec, sub, .. } = self.selected;
        let arguments = self.words.arguments();
        if !spec.subcommands.is_empty()
            && !arguments.is_empty()
            && arguments.literal_at(0).is_none()
        {
            return false;
        }
        crate::registry::layout_is_proven_in(
            spec,
            sub,
            arguments,
            || self.spec_options(),
            |sub| crate::registry::sub_options_at(spec, sub, self.dialect),
        )
    }

    /// The call's argument roles — every `(position, role)` the registry
    /// assigns, in post-head coordinates, sorted by position (the roles one
    /// position carries in [`ArgRole::ALL`] order): the clause grammar, the
    /// resolver or the static table, repeated tails, option values and
    /// command prefixes, as
    /// [`crate::CommandRegistry::arg_indices_for_role_words`] answers them
    /// role by role, for the command or subcommand (or instance method) this
    /// resolution selected under its [`Self::dialect`].
    ///
    /// `None` when the words cannot prove the layout — a `{*}` expansion, a
    /// computed subcommand word, or a computed word where an option a
    /// resolver reads could stand. A computed ordinary operand is fine: it
    /// occupies one position. An empty call is read, not abstained on.
    #[must_use]
    pub fn arg_roles(&self) -> Option<Vec<(usize, ArgRole)>> {
        let arguments = self.words.arguments();
        let count = arguments.exact_argv_len()?;
        if matches!(
            self.subcommand,
            SubcommandResolution::Indeterminate { .. }
                | SubcommandResolution::Unknown { .. }
                | SubcommandResolution::Ambiguous { .. }
        ) {
            return None;
        }
        let (mut selected, complete) = self.authored_source_argument_roles();
        if !complete {
            // Position-only legacy hooks can read inert payloads only after the
            // selected option owner proves that no payload selects a layout.
            if !self.layout_is_proven()
                || self.semantics.clause_grammar.is_some()
                || self.semantics.frame_effect.is_some()
                || self.semantics.arg_role_count_resolver.is_some()
                || self.semantics.arg_role_layout_resolver.is_some()
            {
                return None;
            }
            let spellings = self.placeholder_spellings()?;
            selected =
                self.semantics.arg_role_resolver?(spellings.get(self.semantics.argument_offset..)?);
            let mut complete = true;
            self.extend_repeated_roles(&mut selected, &mut complete, false, None);
            selected.extend(
                self.semantics
                    .options
                    .value_roles(arguments.slice_from(self.semantics.argument_offset))?
                    .into_iter()
                    .map(|(at, role)| u8::try_from(at).ok().map(|at| (at, role)))
                    .collect::<Option<Vec<_>>>()?,
            );
            if !complete {
                return None;
            }
        }
        let SelectedDescriptors {
            spec,
            sub,
            instance,
        } = self.selected;
        let pattern_selected = !instance
            && sub.is_none()
            && (spec.pattern_arg_resolver.is_some() || spec.option_selects_pattern_language());
        let case_body_allowed = instance
            || sub.is_some()
            || spec.case_list.is_none()
            || self.case_invocation().is_some();
        let mut roles = selected
            .into_iter()
            .filter(|(_, role)| {
                *role != ArgRole::CommandPrefix
                    && (*role != ArgRole::Body || case_body_allowed)
                    && (*role != ArgRole::Pattern || !pattern_selected)
            })
            .map(|(at, role)| (usize::from(at) + self.semantics.argument_offset, role))
            .collect::<Vec<_>>();
        if pattern_selected {
            roles.extend(
                self.pattern_args()
                    .into_iter()
                    .map(|pattern| (usize::from(pattern.index), ArgRole::Pattern)),
            );
        }
        let spellings = self.placeholder_spellings()?;
        roles.extend(
            self.command_prefixes_over(&spellings)
                .into_iter()
                .map(|(at, _)| (at, ArgRole::CommandPrefix)),
        );
        roles.retain(|(at, _)| *at < count);
        crate::registry::sort_role_table(&mut roles);
        Some(roles)
    }

    /// The command-prefix positions and appended arities over the placeholder
    /// spellings, with this resolution's word facts behind them so a
    /// literal-sensitive resolver abstains on a computed word.
    fn command_prefixes_over(
        &self,
        spellings: &[&'w str],
    ) -> Vec<(usize, crate::arg_role::AppendedArity)> {
        let arguments = self.words.arguments();
        let words: Vec<InvocationWord<'w>> = (0..spellings.len())
            .map(|index| arguments.get(index).unwrap_or(InvocationWord::Opaque))
            .collect();
        let Some(arguments) = CommandPrefixArguments::structured(spellings, &words) else {
            return Vec::new();
        };
        let SelectedDescriptors { spec, sub, .. } = self.selected;
        crate::registry::command_prefixes_in(spec, sub, arguments)
    }

    /// The call's pattern-bearing arguments and the language each is written
    /// in — [`crate::CommandRegistry::pattern_args_words_for_dialect`]
    /// re-keyed on the resolution. Empty when the command declares none, for
    /// an instance method (the class command's pattern tables are not the
    /// method's), and when the words cannot prove the layout that decides
    /// them.
    #[must_use]
    pub fn pattern_args(&self) -> Vec<crate::patterns::PatternArg> {
        let SelectedDescriptors {
            spec,
            sub,
            instance,
        } = self.selected;
        if instance || !self.layout_is_proven() {
            return Vec::new();
        }
        let Some(spellings) = self.placeholder_spellings() else {
            return Vec::new();
        };
        crate::registry::pattern_args_in(
            spec,
            sub,
            &spellings,
            || self.spec_options(),
            self.dialect,
            || {
                self.authored_source_argument_roles()
                    .0
                    .into_iter()
                    .filter(|(_, role)| *role == ArgRole::Pattern)
                    .map(|(at, _)| usize::from(at) + self.semantics.argument_offset)
                    .collect()
            },
        )
    }

    /// The call read as a case list — its subject, clause-list or inline
    /// clauses, and match mode — with the inline clauses when the call writes
    /// them inline: `CaseListSpec::invocation` and `inline_clauses` re-keyed
    /// on the resolution, over the options available at its
    /// [`Self::dialect`].
    ///
    /// `None` when the command declares no case list, the words do not read
    /// as one, or the reading depends on a computed word's value: it must
    /// hold whether each computed word is an operand or a dash word — in the
    /// option run always, and at a clause start where the descriptor declares
    /// per-clause flags (without them a clause starts with its pattern).
    #[must_use]
    pub fn case_invocation(&self) -> Option<(CaseInvocation, Vec<InlineCaseClause>)> {
        let SelectedDescriptors { spec, instance, .. } = self.selected;
        if instance {
            return None;
        }
        let case = spec.case_list?;
        let spellings = self.placeholder_spellings()?;
        let options = self.spec_options();
        let arguments = self.words.arguments();
        let values = (0..arguments.exact_argv_len()?)
            .map(|at| arguments.literal_at(at))
            .collect::<Vec<_>>();
        let invocation = case.invocation_values(&values, &options, self.dialect)?;
        let clauses = match invocation.inline_clause_start {
            Some(start) => case.inline_clauses(&spellings, start)?,
            None => Vec::new(),
        };
        let arguments = self.words.arguments();
        let dashed: Vec<&str> = spellings
            .iter()
            .enumerate()
            .map(|(index, spelling)| match arguments.get(index) {
                Some(InvocationWord::Dynamic) => "-",
                _ => spelling,
            })
            .collect();
        if dashed != spellings {
            if case.invocation(&dashed, &options, self.dialect) != Some(invocation) {
                return None;
            }
            if !case.clause_flags.is_empty()
                && let Some(start) = invocation.inline_clause_start
                && case.inline_clauses(&dashed, start).as_ref() != Some(&clauses)
            {
                return None;
            }
        }
        Some((invocation, clauses))
    }

    /// The frame the call crosses into, and the operands that act there — the
    /// command's `FrameEffectSpec` read over the call's words under its
    /// [`Self::dialect`]'s release: `upvar`'s level decided by argument-count
    /// parity, `uplevel`'s by the leading word. The operands are the words
    /// after the level word.
    ///
    /// `None` when the command crosses no frame, or when a `{*}` expansion
    /// makes the level word's presence unknown. A computed level word is
    /// [`FrameLevel::Dynamic`], not an abstention, and so is a level whose
    /// spelling the releases read differently when the dialect names none.
    #[must_use]
    pub fn frame_effect(&self) -> Option<(FrameLevel, Vec<OperandId>)> {
        let frame = self.semantics.frame_effect?;
        let offset = self.semantics.argument_offset;
        let arguments = self.words.arguments().slice_from(offset);
        let len = arguments.exact_argv_len()?;
        let taken = frame.level_word_len_for_arguments(arguments)?;
        let level = if taken == 0 {
            FrameLevel::DEFAULT
        } else {
            arguments
                .literal_at(0)
                .and_then(|word| {
                    if let Some(dialect) = arguments.dialect() {
                        FrameLevel::parse_for_dialect(word, dialect)
                    } else {
                        FrameLevel::parse_for(word, self.tcl_version())
                    }
                })
                .unwrap_or(FrameLevel::Dynamic)
        };
        Some((
            level,
            (taken..len).map(|at| OperandId(offset + at)).collect(),
        ))
    }

    /// The type this call's result is represented as —
    /// `CommandSpec::return_type_for_call` re-keyed on the resolution: the
    /// selected subcommand's (or instance method's) declared type for a
    /// command with subcommands, else the command's return-type hook over the
    /// words, else the command's declared type.
    ///
    /// `None` when the type is unknown for this call — a subcommand the call
    /// does not select, a hook that cannot tell (a computed word where a
    /// switch could stand), or a `{*}` expansion under a hook.
    #[must_use]
    pub fn return_type(&self) -> Option<TclType> {
        self.semantics.return_type
    }

    /// Validate registry-declared relationships between literal arguments.
    ///
    /// An absent descriptor is a conservative abstention: it is never treated
    /// as proof that arbitrary command arguments are valid.
    #[must_use]
    pub fn validate_literal_arguments(&self) -> Option<LiteralArgumentValidation> {
        self.semantics
            .literal_argument_validator
            .map(|validator| validator(self.words.arguments()))
    }

    fn extend_repeated_roles(
        &self,
        roles: &mut Vec<(u8, ArgRole)>,
        complete: &mut bool,
        successful: bool,
        frame_layout: Option<crate::frame_effect::FrameArgumentResolution>,
    ) {
        if self.semantics.repeated_args.is_empty() {
            return;
        }
        let arguments = self
            .words
            .arguments()
            .slice_from(self.semantics.argument_offset);
        let Some(count) = arguments.exact_argv_len() else {
            *complete = false;
            return;
        };
        for layout in self.semantics.repeated_args {
            let indices = if layout.optional_leading_word {
                match frame_layout.or_else(|| {
                    self.semantics.frame_effect.map(|effect| {
                        if successful {
                            effect.successful_layout(arguments).layout
                        } else {
                            effect.resolve_arguments(arguments)
                        }
                    })
                }) {
                    Some(crate::frame_effect::FrameArgumentResolution::Valid {
                        level_word_len,
                        ..
                    }) => layout.indices_with_leading_word(count, level_word_len),
                    Some(crate::frame_effect::FrameArgumentResolution::Invalid) => continue,
                    _ => {
                        *complete = false;
                        continue;
                    }
                }
            } else {
                layout.indices(count)
            };
            for index in indices {
                let Ok(index) = u8::try_from(index) else {
                    *complete = false;
                    continue;
                };
                if !roles.contains(&(index, layout.role)) {
                    roles.push((index, layout.role));
                }
            }
        }
        roles.sort_by_key(|(index, _)| *index);
    }

    /// Materialise the selected positional, frame and repeated argument roles.
    ///
    /// Indices are relative to [`InvocationSemantics::argument_offset`]. The
    /// boolean records whether literal-dependent layout selection is complete.
    /// This is the shared projection used by owned facts and compatibility
    /// role queries; selecting a command form must not change their answers.
    #[must_use]
    pub fn argument_roles(&self) -> (Vec<(u8, ArgRole)>, bool) {
        self.argument_roles_on_edge(false)
    }

    /// Argument-role projection on successful completion. A shape eliminated
    /// by grammar error cannot contribute roles to this normal edge.
    #[must_use]
    pub fn argument_roles_after_success(&self) -> (Vec<(u8, ArgRole)>, bool) {
        self.argument_roles_on_edge(true)
    }

    fn non_frame_argument_roles(&self) -> (Vec<(u8, ArgRole)>, bool) {
        if self.semantics.arg_role_resolver_input() == ArgRoleResolverInput::ConflictingResolvers {
            return (Vec::new(), false);
        }
        if self.semantics.clause_grammar.is_some() {
            let Some(Ok(plan)) = self.clause_walk() else {
                return (self.semantics.arg_roles.to_vec(), false);
            };
            let mut roles = self.semantics.arg_roles.to_vec();
            let mut complete = true;
            for (at, role) in plan.roles {
                if let Some(at) = at
                    .checked_sub(self.semantics.argument_offset)
                    .and_then(|at| u8::try_from(at).ok())
                {
                    if !roles.contains(&(at, role)) {
                        roles.push((at, role));
                    }
                } else {
                    complete = false;
                }
            }
            return (roles, complete);
        }
        if let Some(resolver) = self.semantics.arg_role_layout_resolver {
            resolver(
                self.words
                    .arguments()
                    .slice_from(self.semantics.argument_offset),
                self.semantics.options,
            )
            .map_or_else(
                || (self.semantics.arg_roles.to_vec(), false),
                |roles| (roles, true),
            )
        } else if let Some(resolver) = self.semantics.arg_role_count_resolver {
            self.words
                .arguments()
                .exact_argv_len()
                .and_then(|count| count.checked_sub(self.semantics.argument_offset))
                .map_or_else(
                    || (self.semantics.arg_roles.to_vec(), false),
                    |count| (resolver(count), count <= usize::from(u8::MAX) + 1),
                )
        } else {
            match self.semantics.arg_role_resolver {
                Some(resolver) => self
                    .words
                    .arguments()
                    .literal_values()
                    .and_then(|arguments| {
                        arguments
                            .get(self.semantics.argument_offset..)
                            .map(resolver)
                    })
                    .map_or_else(
                        || (self.semantics.arg_roles.to_vec(), false),
                        |roles| (roles, true),
                    ),
                None => (self.semantics.arg_roles.to_vec(), true),
            }
        }
    }

    /// Authored source grammar, independent of native procedure installation.
    /// Options, frame selectors and unknown cardinality retain their ordinary
    /// role uncertainty. Procedure roles come from the selected authored
    /// descriptor; these possible positions grant neither executable facts,
    /// native parameter acceptance nor a successful definition.
    #[must_use]
    pub fn authored_source_argument_roles(&self) -> (Vec<(u8, ArgRole)>, bool) {
        // Implementation contract: naming.vendor.original-registry-metadata
        // docs/design/analysis/name-resolution-proofs/vendor-original-registry-metadata.md
        self.argument_roles_with_native_definition(false, false)
    }

    /// Source-only procedure declaration shape from the selected descriptor.
    /// Original argv cardinality and authored role consensus stay mandatory;
    /// native parameter acceptance and successful definition remain separate.
    #[must_use]
    pub fn authored_source_procedure_arguments(&self) -> Option<AuthoredSourceProcedureArguments> {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        self.semantics.procedure_definition?;
        if self.semantics.frame_effect.is_some()
            || !self
                .semantics
                .traits
                .contains(Traits::DEFINES_PROCEDURE | Traits::DEFERS_BODY)
            || !self
                .semantics
                .arity
                .accepts(self.argument_count_for_arity()?)
        {
            return None;
        }
        let count = self.words.arguments().exact_argv_len()?;
        let (roles, complete) = self.authored_source_argument_roles();
        if !complete {
            return None;
        }
        let ordinal = |wanted| {
            let mut positions = roles.iter().filter_map(|&(position, role)| {
                (role == wanted).then_some(self.semantics.argument_offset + usize::from(position))
            });
            let first = positions.next()?;
            (first < count && positions.next().is_none()).then_some(first)
        };
        Some(AuthoredSourceProcedureArguments {
            name: ordinal(ArgRole::Name)?,
            parameters: ordinal(ArgRole::ParamList)?,
            body: ordinal(ArgRole::Body)?,
        })
    }

    /// Readonly roles in an independently retained Logical source model.
    /// Frame positions use only unanimous existing authored Tcl layouts over
    /// the same structured argv. Dynamic or divergent selectors stay unknown.
    /// Callers must retain the complete positive Logical input; this purpose
    /// grants no Native grammar, entered frame, effects or successful handler.
    #[must_use]
    pub fn authored_logical_source_argument_roles(&self) -> (Vec<(u8, ArgRole)>, bool) {
        // naming.source.original-produced-command-prefix
        // docs/design/analysis/name-resolution-proofs/original-produced-command-prefix.md
        let layout = self
            .semantics
            .frame_effect
            .map(|effect| effect.logical_source_layout(self.words.arguments()));
        self.argument_roles_with_frame_layout(false, false, layout)
    }

    /// Timing of an independently selected original script/container ordinal.
    /// This does not identify a body role, entered callback or executing frame.
    #[must_use]
    pub fn authored_source_script_timing_at(&self, argument: usize) -> Option<crate::ScriptTiming> {
        self.semantics.script_metadata.timing_at(self, argument)
    }

    /// Variable-name option scope at an original effective argument ordinal.
    /// The selected availability, prefix grammar and value widths are shared
    /// with roles; computed options or incomplete layouts refuse the query.
    /// This grants no variable lookup, store, entered frame or allocation.
    #[must_use]
    pub fn authored_source_option_variable_scope_at(
        &self,
        argument: usize,
    ) -> Option<crate::VariableScope> {
        let offset = self.semantics.argument_offset;
        let relative = argument.checked_sub(offset)?;
        let occurrences = self
            .semantics
            .options
            .prefix_occurrences(self.words.arguments().slice_from(offset))?;
        occurrences
            .iter()
            .find(|option| option.values.contains(&relative))?
            .option?
            .value_variable_scope()
    }

    /// Executable source-script positions from the selected descriptors and
    /// timing grammar. Reference-only positions, unresolved layouts and option
    /// widths decline. No reached callback, body frame or Normal is issued.
    #[must_use]
    pub fn authored_source_script_arguments(&self) -> Option<Vec<usize>> {
        let (roles, complete) = self.authored_source_argument_roles();
        self.executable_source_script_arguments(&roles, complete)
    }

    /// Executable source positions under the separate positively retained
    /// Logical role purpose. Native frame grammar and execution remain unknown.
    #[must_use]
    pub fn authored_logical_source_script_arguments(&self) -> Option<Vec<usize>> {
        let (roles, complete) = self.authored_logical_source_argument_roles();
        self.executable_source_script_arguments(&roles, complete)
    }

    /// Plain script or command-prefix positions from the selected source
    /// role, option and timing descriptors. Lambda lists retain their separate
    /// grammar; reference-only operands and unknown layouts remain excluded.
    /// This grants no callback entry, frame, native value or normal completion.
    #[must_use]
    pub fn authored_source_plain_script_arguments(&self) -> Option<Vec<usize>> {
        let (roles, complete) = self.authored_source_argument_roles();
        self.semantics
            .script_metadata
            .plain_script_arguments(self, &roles, complete)
    }

    /// Plain script or command-prefix positions under the separate positively
    /// retained Logical role purpose. Lambda grammar and Native frame or effect
    /// facts remain independent of this readonly source projection.
    #[must_use]
    pub fn authored_logical_source_plain_script_arguments(&self) -> Option<Vec<usize>> {
        let (roles, complete) = self.authored_logical_source_argument_roles();
        self.semantics
            .script_metadata
            .plain_script_arguments(self, &roles, complete)
    }

    fn executable_source_script_arguments(
        &self,
        roles: &[(u8, ArgRole)],
        complete: bool,
    ) -> Option<Vec<usize>> {
        Some(
            self.semantics
                .script_metadata
                .script_arguments(self, roles, complete)?
                .into_iter()
                .filter_map(|(ordinal, timing)| {
                    (timing != crate::hover::ScriptTiming::ReferenceOnly).then_some(ordinal)
                })
                .collect(),
        )
    }

    /// Executable command-prefix positions from the selected authored source
    /// grammar. Unknown payloads keep their slots; selectors, option widths,
    /// expanded cardinality and reference-only timing retain their refusals.
    /// Effective ordinals include selected subcommands and captured prefixes.
    /// This grants no dispatch, callback entry, native value or normal completion.
    #[must_use]
    pub fn authored_source_command_prefix_arguments(
        &self,
    ) -> Option<Vec<(usize, crate::AppendedArity)>> {
        // naming.source.original-structured-script-timing
        // docs/design/analysis/name-resolution-proofs/original-structured-script-timing.md
        let (roles, complete) = self.authored_source_argument_roles();
        self.semantics
            .script_metadata
            .prefix_arguments(self, &roles, complete)
    }

    /// ASCII list value of a selected list-arguments descriptor for readonly
    /// source grammar. Every element must retain an actual known ASCII value;
    /// unknown, expanded or opaque elements and other dialects decline. The
    /// shared Tcl list serializer owns quoting and separators. No native object,
    /// result allocation, effect closure or successful handler entry follows.
    #[must_use]
    pub fn authored_source_ascii_list_result(&self) -> Option<Vec<u8>> {
        // naming.source.original-structured-script-timing
        // docs/design/analysis/name-resolution-proofs/original-structured-script-timing.md
        let arguments = self.words.arguments();
        let dialect = arguments.dialect()?;
        if self.semantics.native_result
            != Some(crate::native_result::NativeResultContract::ListArguments { from: 0 })
            || self.semantics.argument_offset != 0
            || dialect.family() != Some(tcl_dialect::model::Family::Tcl)
            || dialect.native_string_protocol().is_none()
            || !matches!(self.subcommand, SubcommandResolution::NotApplicable)
        {
            return None;
        }
        let count = arguments.exact_argv_len()?;
        let mut result = Vec::new();
        for ordinal in 0..count {
            let bytes = match arguments.get(ordinal)? {
                crate::InvocationWord::KnownBytes(bytes) => bytes,
                crate::InvocationWord::Literal(value) => value.as_bytes(),
                _ => return None,
            };
            if !bytes.is_ascii() || bytes.contains(&0) {
                return None;
            }
            if ordinal != 0 {
                result.push(b' ');
            }
            tcl_syntax::list::append_list_element(&mut result, bytes, ordinal == 0);
        }
        Some(result)
    }

    /// Pattern languages and effective argument ordinals from the already
    /// selected descriptors and available option grammar. Unknown selectors,
    /// option widths and expanded cardinality decline. Ordinary payloads keep
    /// their unknown values. This readonly source projection grants no handler,
    /// evaluated pattern, normal completion or edit equivalence.
    #[must_use]
    pub fn authored_source_pattern_arguments(&self) -> Option<Vec<crate::patterns::PatternArg>> {
        // naming.core.original-pattern-retained-context
        // docs/design/analysis/name-resolution-proofs/original-pattern-retained-context.md
        if !matches!(
            self.subcommand,
            SubcommandResolution::NotApplicable
                | SubcommandResolution::Exact(_)
                | SubcommandResolution::UniquePrefix(_)
        ) {
            return None;
        }
        let arguments = self.words.arguments();
        let count = arguments.exact_argv_len()?;
        let selected = self.semantics.script_metadata;
        if let Some(resolve) = selected.command.pattern_arg_resolver {
            // Paired command resolvers own full post-head ordinals. A selected
            // member cannot borrow that command-level option layout.
            if selected.subcommand.is_some() || self.semantics.argument_offset != 0 {
                return None;
            }
            let options = self.semantics.options;
            let prefix = options.prefix_occurrences(arguments)?;
            let spellings = (0..count)
                .map(|index| arguments.literal_at(index).unwrap_or_default())
                .collect::<Vec<_>>();
            for occurrence in prefix {
                if occurrence.option.is_some_and(|option| {
                    option.value_word_count(&spellings, occurrence.argument_index)
                        != occurrence.values.len()
                }) {
                    return None;
                }
            }
            let available = options.available().collect::<Vec<_>>();
            let mut patterns = resolve(
                &spellings,
                crate::patterns::PatternArgResolverContext {
                    options: &available,
                    reserved_trailing_words: options.reserved_trailing_words,
                },
            );
            patterns.retain(|pattern| usize::from(pattern.index) < count);
            return Some(patterns);
        }
        let Some(kind) = selected
            .subcommand
            .and_then(|sub| sub.pattern_type)
            .or(selected.command.pattern_type)
        else {
            return Some(Vec::new());
        };
        let (roles, complete) = self.authored_source_argument_roles();
        if !complete {
            return None;
        }
        roles
            .into_iter()
            .filter(|(_, role)| *role == ArgRole::Pattern)
            .map(|(index, _)| {
                let index = self
                    .semantics
                    .argument_offset
                    .checked_add(usize::from(index))?;
                (index < count).then_some(crate::patterns::PatternArg {
                    index: u8::try_from(index).ok()?,
                    kind,
                })
            })
            .collect()
    }

    /// Format operands from the same selected descriptors and source-role
    /// grammar. Complete effective ordinals include captured prefixes and
    /// selected subcommands. This is readonly metadata, not entered dispatch,
    /// template value validation, successful completion or edit permission.
    #[must_use]
    pub fn authored_source_format_arguments(&self) -> Option<Vec<crate::FormatStringArg>> {
        // naming.core.original-inlay-retained-context
        // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
        if !matches!(
            self.subcommand,
            SubcommandResolution::NotApplicable
                | SubcommandResolution::Exact(_)
                | SubcommandResolution::UniquePrefix(_)
        ) {
            return None;
        }
        let selected = self.semantics.script_metadata;
        let Some(kind) = selected
            .subcommand
            .and_then(|sub| sub.format_string_type)
            .or(selected.command.format_string_type)
        else {
            return Some(Vec::new());
        };
        let count = self.words.arguments().exact_argv_len()?;
        let (roles, complete) = self.authored_source_argument_roles();
        if !complete {
            return None;
        }
        let mut formats = roles
            .into_iter()
            .filter(|(_, role)| matches!(role, ArgRole::FormatString | ArgRole::ScanFormat))
            .filter_map(|(index, role)| {
                let index = self
                    .semantics
                    .argument_offset
                    .checked_add(usize::from(index))?;
                (index < count).then_some(crate::FormatStringArg {
                    index,
                    kind,
                    scan: role == ArgRole::ScanFormat,
                })
            })
            .collect::<Vec<_>>();
        formats.sort_unstable_by_key(|format| format.index);
        formats.dedup();
        Some(formats)
    }

    /// Selected signature and count axis at authentic effective operands.
    /// Dedicated structural arity checkers and unresolved selectors decline.
    /// Package windows remain explicit for consumers that decide floors later.
    #[must_use]
    pub fn authored_source_arity(&self) -> Option<AuthoredSourceArity<'r>> {
        // naming.diagnostic.registry-source-ownership
        // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
        if !matches!(
            self.subcommand,
            SubcommandResolution::NotApplicable
                | SubcommandResolution::Exact(_)
                | SubcommandResolution::UniquePrefix(_)
        ) || self
            .semantics
            .traits
            .contains(Traits::STRUCTURALLY_CHECKED_ARITY)
        {
            return None;
        }
        let selected = self.semantics.script_metadata;
        let missing_subcommand = !selected.command.subcommands.is_empty()
            && self.words.arguments().is_empty()
            && selected.command.constructor_prefix_words().is_none();
        let windows = selected
            .subcommand
            .map_or(selected.command.arity_windows, |sub| sub.arity_windows);
        let synopsis = selected.subcommand.map_or_else(
            || {
                selected
                    .command
                    .primary_synopsis(self.semantics.options.availability.package_version)
            },
            SubCommand::primary_synopsis,
        );
        Some(AuthoredSourceArity {
            command: selected.command,
            subcommand: selected.subcommand,
            arity: self.semantics.arity,
            windows,
            missing_subcommand,
            synopsis,
            count: self.authored_source_count_for_arity(self.semantics.arity)?,
        })
    }

    /// Recount this same retained invocation under a selected signature window.
    /// The count axis belongs to that window; a cached fallback count cannot
    /// be reused when a version selects a different axis or option layout.
    #[must_use]
    pub fn authored_source_count_for_arity(&self, arity: Arity) -> Option<InvocationArgumentCount> {
        count_invocation_argument_layout(
            arity,
            self.words.arguments(),
            self.semantics.argument_offset,
            self.semantics.options,
        )
    }

    /// Selected Apply lambda grammar at its declared `LambdaLiteral` operand.
    /// An unknown/expanded lambda position cannot establish trailing ordinals.
    /// Dynamic values retain their slots, while a trailing expansion remains
    /// an unknown count. This establishes no actual lambda or invocation.
    #[must_use]
    pub fn authored_source_lambda_call(&self) -> Option<AuthoredSourceLambdaCall> {
        // naming.diagnostic.registry-source-ownership
        // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
        if !matches!(
            self.subcommand,
            SubcommandResolution::NotApplicable
                | SubcommandResolution::Exact(_)
                | SubcommandResolution::UniquePrefix(_)
        ) || self.semantics.analyser_hook != Some(crate::hooks::AnalyserHookId::Apply)
        {
            return None;
        }
        let mut lambdas = self
            .semantics
            .arg_roles
            .iter()
            .filter_map(|&(index, role)| {
                (role == ArgRole::LambdaLiteral).then_some(usize::from(index))
            });
        let lambda_argument = self
            .semantics
            .argument_offset
            .checked_add(lambdas.next()?)?;
        if lambdas.next().is_some()
            || !self
                .words
                .arguments()
                .get(lambda_argument)?
                .has_exactly_one_argv_entry()
        {
            return None;
        }
        let count = count_invocation_argument_layout(
            Arity::any(),
            self.words.arguments(),
            lambda_argument.checked_add(1)?,
            self.semantics.options,
        )?;
        Some(AuthoredSourceLambdaCall {
            lambda_argument,
            count,
        })
    }

    /// Variable-name operands of the selected source grammar. Alias-pair
    /// remote operands can be computed names; only their local writes carry
    /// the name/value-confusion purpose. No actual variable cell is selected.
    #[must_use]
    pub fn authored_source_variable_name_arguments(&self) -> Vec<(usize, ArgRole)> {
        let (roles, complete) = self.authored_source_argument_roles();
        if !complete {
            return Vec::new();
        }
        let local_alias = self
            .semantics
            .frame_effect
            .is_some_and(|effect| effect.layout == crate::frame_effect::FrameArgLayout::AliasPairs);
        roles
            .into_iter()
            .filter(|(_, role)| {
                *role == ArgRole::VarWrite || (!local_alias && *role == ArgRole::VarRead)
            })
            .filter_map(|(index, role)| {
                Some((
                    self.semantics
                        .argument_offset
                        .checked_add(usize::from(index))?,
                    role,
                ))
            })
            .collect()
    }

    /// Name/value or declaration-tail source positions from the selected
    /// descriptor. Read-modify-write and whole-array forms are excluded;
    /// unknown roles/cardinality decline. No store or cell value is proved.
    #[must_use]
    pub fn authored_source_assignment_arguments(&self) -> Option<Vec<(usize, Option<usize>)>> {
        // naming.navigation.retained-path-source-inventory
        // docs/design/analysis/name-resolution-proofs/retained-path-source-inventory.md
        use crate::{ArgRole, Traits};
        let count = self.words.arguments().exact_argv_len()?;
        let (roles, complete) = self.authored_source_argument_roles();
        if !complete {
            return None;
        }
        if self
            .semantics
            .traits
            .intersects(Traits::READS_BEFORE_WRITE | Traits::WHOLE_ARRAY_ARG)
        {
            return Some(Vec::new());
        }
        let Some(base) = self
            .semantics
            .script_metadata
            .command
            .assigns_variable_at
            .map(usize::from)
        else {
            return Some(Vec::new());
        };
        let writes = roles
            .iter()
            .filter(|(_, role)| *role == ArgRole::VarWrite)
            .map(|(argument, _)| self.semantics.argument_offset + usize::from(*argument))
            .collect::<Vec<_>>();
        let paired_tail = self.semantics.repeated_args.iter().any(|layout| {
            layout.role == ArgRole::VarWrite
                && layout.stride == 2
                && self.semantics.argument_offset + usize::from(layout.start) == base
                && !layout.conditional_binding
        });
        if paired_tail {
            return Some(
                writes
                    .into_iter()
                    .map(|argument| (argument, (argument + 1 < count).then_some(argument + 1)))
                    .collect(),
            );
        }
        Some(
            if self.semantics.repeated_args.is_empty() && writes == [base] && count == base + 2 {
                vec![(base, Some(base + 1))]
            } else {
                Vec::new()
            },
        )
    }

    /// Expression roles and whole-tail grammar from the already selected
    /// descriptor. Unknown selectors, unresolved roles and expanded argv
    /// cardinality withdraw this projection; ordinary unknown values keep
    /// their slots. No native evaluation or brace-rewrite equivalence follows.
    #[must_use]
    pub fn authored_source_expression_arguments(
        &self,
    ) -> Option<AuthoredSourceExpressionArguments> {
        self.source_expression_arguments(self.authored_source_argument_roles())
    }

    /// Expression positions under an independently retained positive Logical
    /// source model. Native frame grammar and execution remain unknown.
    #[must_use]
    pub fn authored_logical_source_expression_arguments(
        &self,
    ) -> Option<AuthoredSourceExpressionArguments> {
        self.source_expression_arguments(self.authored_logical_source_argument_roles())
    }

    fn source_expression_arguments(
        &self,
        (roles, complete): (Vec<(u8, ArgRole)>, bool),
    ) -> Option<AuthoredSourceExpressionArguments> {
        // naming.diagnostic.registry-source-ownership
        // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
        if !matches!(
            self.subcommand,
            SubcommandResolution::NotApplicable
                | SubcommandResolution::Exact(_)
                | SubcommandResolution::UniquePrefix(_)
        ) {
            return None;
        }
        let count = self.words.arguments().exact_argv_len()?;
        if !complete {
            return None;
        }
        let mut arguments = roles
            .into_iter()
            .filter(|(_, role)| *role == ArgRole::Expr)
            .map(|(index, _)| {
                let index = self
                    .semantics
                    .argument_offset
                    .checked_add(usize::from(index))?;
                (index < count).then_some(index)
            })
            .collect::<Option<Vec<_>>>()?;
        arguments.sort_unstable();
        arguments.dedup();
        Some(AuthoredSourceExpressionArguments {
            arguments,
            concatenates: self
                .semantics
                .traits
                .contains(Traits::EXPR_CONCATENATES_ARGS),
        })
    }

    /// Payload slots from the selected Append codegen descriptor and its complete variable
    /// role. Exact argv cardinality is required; dynamic payload values remain
    /// unknown. This is descriptive syntax, not a variable read or rewrite.
    #[must_use]
    pub fn authored_source_append_arguments(&self) -> Option<AuthoredSourceAppendArguments> {
        self.authored_source_append_layout(crate::hooks::CodegenHookId::Append)
    }

    /// Exact list-append source receiver and value slots from its selected
    /// descriptor. This supplies no native receiver, list conversion or write.
    #[must_use]
    pub fn authored_source_list_append_arguments(&self) -> Option<AuthoredSourceAppendArguments> {
        self.authored_source_append_layout(crate::hooks::CodegenHookId::Lappend)
    }

    fn authored_source_append_layout(
        &self,
        hook: crate::hooks::CodegenHookId,
    ) -> Option<AuthoredSourceAppendArguments> {
        if self.semantics.codegen_hook != Some(hook) {
            return None;
        }
        let count = self.words.arguments().exact_argv_len()?;
        let (roles, complete) = self.authored_source_argument_roles();
        if !complete {
            return None;
        }
        let mut variables = roles
            .into_iter()
            .filter_map(|(index, role)| (role == ArgRole::VarWrite).then_some(usize::from(index)));
        let variable = self
            .semantics
            .argument_offset
            .checked_add(variables.next()?)?;
        if variables.next().is_some() || variable >= count {
            return None;
        }
        Some(AuthoredSourceAppendArguments {
            variable,
            values: variable.checked_add(1)?..count,
        })
    }

    /// A selected Unset source shape whose actual dialect option protocol
    /// consumed every operand. Complete role classification preserves fixed C
    /// versus Jim prefix rules; no consumer scans or repeats option spellings.
    #[must_use]
    pub fn authored_source_unset_option_only_arguments(&self) -> Option<std::ops::Range<usize>> {
        if self.semantics.lowering_hook != Some(crate::hooks::LoweringHookId::Unset) {
            return None;
        }
        let count = self.words.arguments().exact_argv_len()?;
        let (roles, complete) = self.authored_source_argument_roles();
        (complete && count > 0 && !roles.iter().any(|(_, role)| *role == ArgRole::VarWrite))
            .then_some(0..count)
    }

    /// Case action positions for original-word quoting advice. The selected
    /// outer grammar retains unknown subject/actions; only option and clause
    /// flag selection need values. This does not validate an action value,
    /// enter a script or establish successful matching/completion.
    #[must_use]
    pub fn authored_source_case_body_arguments(&self) -> Option<Vec<AuthoredSourceCaseBody>> {
        if self.semantics.argument_offset != 0 {
            return None;
        }
        let case = *self.semantics.options.case_list?;
        if !case.warn_unbraced_bodies {
            return Some(Vec::new());
        }
        let arguments = self.words.arguments();
        let count = arguments.exact_argv_len()?;
        let values = (0..count)
            .map(|index| arguments.literal_at(index))
            .collect::<Vec<_>>();
        let options = self.semantics.options.available().collect::<Vec<_>>();
        let invocation = case.source_invocation_values(
            &values,
            &options,
            self.semantics.options.availability.query,
        )?;
        let regexp = invocation.mode == crate::spec::CaseMatchMode::Regexp;
        if let Some(argument) = invocation.clause_list_index {
            return Some(vec![AuthoredSourceCaseBody {
                argument,
                regexp,
                single_block: true,
            }]);
        }
        let start = invocation.inline_clause_start?;
        Some(
            case.inline_clause_positions(&values, start)?
                .into_iter()
                .filter_map(|clause| {
                    let argument = clause.body_index?;
                    (case.fallthrough_body != values.get(argument).copied().flatten()).then_some(
                        AuthoredSourceCaseBody {
                            argument,
                            regexp: regexp || clause.mode == crate::spec::CaseMatchMode::Regexp,
                            single_block: false,
                        },
                    )
                })
                .collect(),
        )
    }

    /// Declared option diagnostics from the already selected source schema.
    /// Exact excluded rows remain explainable; prefixes use the actual admitted
    /// vocabulary. Dynamic selection, unknown widths and reserved data cannot
    /// be skipped to discover a later option. No execution or edit is implied.
    #[must_use]
    pub fn authored_source_diagnostic_options(&self) -> Option<AuthoredSourceOptionScan<'r>> {
        // naming.diagnostic.registry-source-ownership
        // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
        if !matches!(
            self.subcommand,
            SubcommandResolution::NotApplicable
                | SubcommandResolution::Exact(_)
                | SubcommandResolution::UniquePrefix(_)
        ) {
            return None;
        }
        let selected = self.semantics.script_metadata;
        let options = self.semantics.options;
        let mut subcommands = selected
            .subcommand
            .map(|sub| sub.name)
            .into_iter()
            .collect::<Vec<_>>();
        if let Some(sub) = selected
            .subcommand
            .filter(|sub| !sub.sub_subcommands.is_empty())
        {
            let word = self.words.arguments().literal_at(1)?;
            let nested = sub.resolve_sub_subcommand_gated(
                word,
                options.availability.query,
                options.availability.package_version,
            )?;
            subcommands.push(nested.name);
        }
        let mut scan = options.diagnostic_scan(
            self.words
                .arguments()
                .slice_from(self.semantics.argument_offset),
            self.semantics.argument_offset,
            selected
                .subcommand
                .map_or(selected.command.option_placement, |sub| {
                    sub.option_placement
                }),
        )?;
        scan.subcommands = subcommands;
        Some(scan)
    }

    /// Possible option positions for conditional source advice. Selection,
    /// prefixes, value widths, reserved data and terminator availability all
    /// use this invocation's retained context. Dynamic operands preserve every
    /// admitted continuation; no Normal completion or option value is proved.
    #[must_use]
    pub fn authored_source_possible_option_arguments(
        &self,
    ) -> Option<AuthoredSourceOptionArguments> {
        let scan = self.authored_source_diagnostic_options()?;
        if !scan.accepts_terminator {
            return None;
        }
        let selected = self.semantics.script_metadata;
        let arguments = self.semantics.options.possible_option_arguments(
            self.words
                .arguments()
                .slice_from(self.semantics.argument_offset),
            selected
                .subcommand
                .map_or(selected.command.option_placement, |sub| {
                    sub.option_placement
                }),
        )?;
        Some(AuthoredSourceOptionArguments {
            arguments: arguments
                .into_iter()
                .map(|index| self.semantics.argument_offset + index)
                .collect(),
            subcommands: scan.subcommands,
        })
    }

    /// Relationship facts from the shared actual source option topology.
    /// Unavailable/unknown/dynamic option boundaries preserve only established
    /// positive observations. Only a complete classified vector can establish
    /// an absent term. No runtime substitution, handler or repair follows.
    #[must_use]
    pub fn authored_source_option_relationships(
        &self,
    ) -> Option<AuthoredSourceOptionRelationships<'r>> {
        // naming.diagnostic.registry-source-ownership
        // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
        let scan = self.authored_source_diagnostic_options()?;
        let arguments = self.words.arguments();
        let count = arguments.exact_argv_len()?;
        let selected = self.semantics.script_metadata;
        let (base, forms, constraints, surface) = selected.subcommand.map_or(
            (
                selected.command.option_relations,
                selected.command.command_forms,
                selected.command.constraints,
                selected.command.surface,
            ),
            |sub| {
                (
                    sub.option_relations,
                    sub.subcommand_forms,
                    sub.constraints,
                    sub.surface.or(selected.command.surface),
                )
            },
        );
        let relations = base
            .iter()
            .chain(
                forms
                    .iter()
                    .filter(|form| self.form.is_some_and(|selected| selected.name == form.name))
                    .flat_map(|form| form.option_relations.iter()),
            )
            .filter(|relation| {
                relation.supports_dialect(self.semantics.options.availability.query, surface)
            })
            .collect();
        let mut consumed = std::collections::BTreeSet::new();
        let mut complete = true;
        for option in &scan.options {
            if !option.available {
                complete = false;
            }
            consumed.insert(option.argument);
            if let Some(values) = &option.values {
                consumed.extend(values.clone());
            } else {
                complete = false;
            }
        }
        let bound = match scan.boundary {
            AuthoredSourceOptionBoundary::Positional(_) | AuthoredSourceOptionBoundary::End => {
                count
            }
            AuthoredSourceOptionBoundary::Terminator(index) => {
                consumed.insert(index);
                count
            }
            AuthoredSourceOptionBoundary::Dynamic(index)
            | AuthoredSourceOptionBoundary::Unknown(index)
            | AuthoredSourceOptionBoundary::Ambiguous {
                argument: index, ..
            } => {
                complete = false;
                index
            }
            AuthoredSourceOptionBoundary::Indeterminate => {
                complete = false;
                0
            }
        };
        let positionals = (self.semantics.argument_offset..bound)
            .filter(|index| !consumed.contains(index))
            .collect::<Vec<_>>();
        complete &= (self.semantics.argument_offset..count)
            .all(|index| arguments.literal_at(index).is_some());
        Some(AuthoredSourceOptionRelationships {
            scan,
            positionals,
            complete,
            relations,
            constraints,
        })
    }

    /// Formatter presentation for an actual authored Body operand. The
    /// independently selected subcommand owns its local presentation table;
    /// unknown role/cardinality remains absent. This is a style preference,
    /// not script entry, runtime equivalence or rewrite permission.
    #[must_use]
    pub fn authored_source_argument_presentation(
        &self,
        argument: usize,
    ) -> Option<crate::presentation::ArgPresentation> {
        let count = self.words.arguments().exact_argv_len()?;
        if argument >= count {
            return None;
        }
        let local = argument.checked_sub(self.semantics.argument_offset)?;
        let (roles, complete) = self.authored_source_argument_roles();
        if !complete
            || !roles
                .iter()
                .any(|&(index, role)| usize::from(index) == local && role == ArgRole::Body)
        {
            return None;
        }
        let selected = self.semantics.script_metadata;
        let table = selected
            .subcommand
            .map_or(selected.command.arg_presentation, |subcommand| {
                subcommand.arg_presentation
            });
        Some(
            table
                .iter()
                .find(|&&(index, _)| usize::from(index) == local)
                .map_or_else(
                    crate::presentation::ArgPresentation::default,
                    |&(_, presentation)| presentation,
                ),
        )
    }

    /// Possible case-list layout from this already selected source descriptor.
    /// Dynamic subjects retain their unknown value. Exact cardinality, option
    /// availability and clause grammar use the same descriptor as execution;
    /// this grants no matching, entered body or normal-completion authority.
    #[must_use]
    pub fn authored_source_case_invocation(
        &self,
    ) -> Option<(crate::CaseListSpec, crate::spec::CaseInvocation)> {
        // Implementation contract: naming.source.original-editor-body-structure
        // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md

        if self.semantics.argument_offset != 0 {
            return None;
        }
        let case = *self.semantics.options.case_list?;
        let arguments = self.words.arguments();
        let count = arguments.exact_argv_len()?;
        let values = (0..count)
            .map(|index| {
                arguments.literal_at(index).or_else(|| {
                    let bytes = arguments.native_bytes_at(index)?;
                    if bytes.contains(&0) {
                        return None;
                    }
                    std::str::from_utf8(bytes).ok()
                })
            })
            .collect::<Vec<_>>();
        let options = self.semantics.options.available().collect::<Vec<_>>();
        case.invocation_values(&values, &options, self.semantics.options.availability.query)
            .map(|layout| (case, layout))
    }

    /// Incomplete clause-list style from this actual source descriptor and
    /// exact unchanged argv. A complete flag-free odd list may be presented
    /// despite unavailable executable roles; no matching/body/Normal follows.
    #[must_use]
    pub fn authored_source_case_presentation(&self) -> Option<(crate::CaseListSpec, usize)> {
        // Implementation contract: naming.source.original-editor-body-structure
        // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md

        if self.semantics.argument_offset != 0 {
            return None;
        }
        let case = *self.semantics.options.case_list?;
        let arguments = self.words.arguments();
        let count = arguments.exact_argv_len()?;
        let values = (0..count)
            .map(|index| {
                arguments.literal_at(index).or_else(|| {
                    let bytes = arguments.native_bytes_at(index)?;
                    (!bytes.contains(&0)).then_some(())?;
                    std::str::from_utf8(bytes).ok()
                })
            })
            .collect::<Vec<_>>();
        let options = self.semantics.options.available().collect::<Vec<_>>();
        case.presentation_clause_list_argument(
            &values,
            &options,
            self.semantics.options.availability.query,
            arguments.dialect()?.word_values,
        )
        .map(|argument| (case, argument))
    }

    /// Possible receiver naming form from this selected authored source grammar.
    /// Complete authored roles and accepted source cardinality are required;
    /// unknown options, frame selectors and expansion retain their uncertainty.
    /// This does not validate native installation, access or successful execution.
    #[must_use]
    pub fn authored_source_variable_receiver_operand_form(
        &self,
        argument: usize,
    ) -> Option<VariableReceiverOperandForm> {
        // Implementation contract: naming.vendor.original-registry-metadata
        // docs/design/analysis/name-resolution-proofs/vendor-original-registry-metadata.md
        let (roles, complete) = self.authored_source_argument_roles();
        if !complete
            || self
                .argument_count_for_arity()
                .is_none_or(|count| !self.semantics.arity.accepts(count))
        {
            return None;
        }
        selected_variable_receiver_operand_form(
            argument,
            self.semantics.argument_offset,
            &roles,
            self.semantics.traits,
            self.semantics.variable_receivers,
            self.semantics.successful_handler,
            self.semantics.native_compilation,
        )
    }

    /// Diagnose an original literal selector with the already selected table.
    /// Default forms, factories, dynamic selectors and open vocabularies decline.
    /// Only an excluded declared row uses the unfiltered table, to explain its
    /// availability; accepted and ambiguous prefixes use the actual vocabulary.
    #[must_use]
    pub fn authored_source_subcommand_diagnostic(
        &self,
    ) -> Option<AuthoredSourceSubcommandDiagnostic> {
        let command = self.semantics.script_metadata.command;
        if command.allow_unknown_subcommands {
            return None;
        }
        let (SubcommandResolution::Unknown { spelling }
        | SubcommandResolution::Ambiguous { spelling }) = self.subcommand
        else {
            return None;
        };
        let table = command.subcommand_table(
            self.semantics.options.availability.query,
            self.semantics.options.availability.package_version,
            None,
        );
        if let crate::abbrev::KeywordMatch::Ambiguous(candidates) = table.resolve(spelling) {
            return Some(AuthoredSourceSubcommandDiagnostic::Ambiguous { candidates });
        }
        if let crate::abbrev::KeywordMatch::Unique(canonical) =
            command.resolve_subcommand_word(spelling, None, None, None)
        {
            let sub = command.subcommand(canonical)?;
            return Some(AuthoredSourceSubcommandDiagnostic::Disabled {
                canonical,
                surface: sub.surface.or(command.surface),
            });
        }
        Some(AuthoredSourceSubcommandDiagnostic::Unknown {
            candidates: table.names().collect(),
        })
    }

    /// Clause shape from the selected original grammar. Unknown payloads keep
    /// their positions; unknown selectors and expanded counts do not select a
    /// shape. This is source advice, not native execution admission.
    #[must_use]
    pub fn authored_source_clause_shape(&self) -> Option<crate::ClauseShapeError> {
        self.authored_source_clause_issue()
            .map(crate::ClauseShapeIssue::error)
    }

    /// Clause defect and optional repair anchor from this selected grammar.
    /// A diagnostic-only pack error supplies no borrowed stock edit proposal.
    #[must_use]
    pub fn authored_source_clause_issue(&self) -> Option<crate::ClauseShapeIssue> {
        (self.semantics.script_metadata.command.clause_shape_check?)(self.words.arguments())
    }

    /// Lexical context advice from the same selected descriptor and argv.
    /// No execution frame or native return capability is supplied here.
    #[must_use]
    pub fn authored_source_context_gate(&self, in_event_body: bool) -> Option<&'static str> {
        (self.semantics.script_metadata.command.context_gate?)(
            self.words.arguments(),
            in_event_body,
        )
    }

    /// Names of a consecutive optional trailing role run in this exact
    /// selected descriptor. Dynamic layouts decline rather than invent values.
    /// Each returned name is synopsis metadata, not a variable allocation.
    #[must_use]
    pub fn authored_source_optional_trailing_names(&self, role: ArgRole) -> Vec<&'static str> {
        let arguments = self.words.arguments();
        let Some(count) = arguments.exact_argv_len() else {
            return Vec::new();
        };
        if self.semantics.argument_offset != 0 || self.semantics.arg_role_layout_resolver.is_some()
        {
            return Vec::new();
        }
        let roles = if let Some(resolve) = self.semantics.arg_role_count_resolver {
            resolve(count)
        } else if let Some(resolve) = self.semantics.arg_role_resolver {
            let Some(values) = arguments.literal_values() else {
                return Vec::new();
            };
            resolve(&values)
        } else {
            self.semantics.arg_roles.to_vec()
        };
        let ceiling = usize::from(self.semantics.arity.max);
        let run = (count..ceiling)
            .take_while(|index| {
                roles.iter().any(|&(position, selected)| {
                    usize::from(position) == *index && selected == role
                })
            })
            .count();
        self.semantics
            .script_metadata
            .command
            .optional_trailing_arg_names(self.semantics.options.availability.query, None)
            .into_iter()
            .take(run)
            .collect()
    }

    /// Borrow the same selected descriptors for readonly lifecycle and syntax
    /// assistance. Canonical reporting names must not be looked up afresh.
    #[must_use]
    pub const fn authored_source_descriptors(&self) -> AuthoredSourceDescriptors<'r> {
        AuthoredSourceDescriptors {
            command: self.semantics.script_metadata.command,
            subcommand: self.semantics.script_metadata.subcommand,
        }
    }

    /// Exact authored naming layout from the selected descriptors and options.
    /// Unknown control words, expansion cardinality, and optional factory names
    /// decline. This supplies source metadata only, never a published command.
    #[must_use]
    pub fn authored_source_command_publication(&self) -> Option<AuthoredSourceCommandPublication> {
        // naming.source.original-command-name-publications
        // docs/design/analysis/name-resolution-proofs/original-command-name-publications.md
        let descriptors = self.authored_source_descriptors();
        let arguments = self.words.arguments();
        let count = arguments.exact_argv_len()?;
        let offset = self.semantics.argument_offset;
        let (relative, kind) = if let Some(sub) = descriptors.subcommand {
            (
                sub.defines_command_at?,
                AuthoredSourceCommandPublicationKind::Command,
            )
        } else if let Some(argument) = descriptors.command.creates_instance_at {
            // Optional naming/control layouts require their own authored recipe.
            if descriptors.command.arity.min == 0 {
                return None;
            }
            (
                argument,
                AuthoredSourceCommandPublicationKind::Instance {
                    class_name: descriptors
                        .command
                        .object_class
                        .map_or(descriptors.command.name, |class| class.class_name),
                },
            )
        } else {
            if descriptors.command.traits.contains(Traits::IS_OO_METACLASS)
                && descriptors.command.definition_body.is_some()
            {
                return None;
            }
            (
                descriptors.command.defines_command_at?,
                AuthoredSourceCommandPublicationKind::Command,
            )
        };
        let leading = self
            .semantics
            .options
            .leading_word_count(arguments.slice_from(offset))?;
        let argument = offset
            .checked_add(leading)?
            .checked_add(usize::from(relative))?;
        (argument < count).then_some(AuthoredSourceCommandPublication { argument, kind })
    }

    /// Conditional callable class of this selected factory descriptor.
    /// Original operands and exported manufacturer visibility are retained;
    /// this does not prove object creation, completion or method dispatch.
    #[must_use]
    pub fn authored_source_callable_factory_class(&self) -> Option<&'static str> {
        let descriptors = self.authored_source_descriptors();
        if descriptors.subcommand.is_some() {
            return None;
        }
        if let Some(factory) = self.semantics.named_object_factory {
            return Some(factory.class_name());
        }
        let class = descriptors.command.object_class?;
        let method = crate::CommandRegistry::manufacturer_method_for_descriptor(
            descriptors.command,
            self.words.arguments().literal_at(0)?,
        )?;
        (method.visibility == crate::definer::MemberVisibility::Exported)
            .then_some(class.class_name)
    }

    /// Definition vocabulary of an actual authored script operand. Configure
    /// operations enter that vocabulary only for the body directly following
    /// their selected target; an inline method body remains ordinary Tcl.
    /// This projection supplies neither a runtime frame nor target presence.
    #[must_use]
    pub fn authored_source_definition_body_grammar(
        &self,
    ) -> Option<&'static crate::definer::DefinitionBodyGrammar> {
        // naming.core.original-inlay-retained-context
        // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
        let grammar = self.authored_source_descriptors().command.definition_body?;
        let transitions = self.state_transitions();
        let target = transitions
            .facts()
            .iter()
            .find_map(|fact| match &fact.transition {
                crate::StateTransition::ObjectDispatch(
                    crate::ObjectDispatchTransition::Configure { target, .. },
                ) => target.argument_index(),
                _ => None,
            });
        if let Some(target) = target {
            let body = target.checked_add(1)?;
            let (roles, complete) = self.authored_source_argument_roles();
            return (complete
                && roles.iter().any(|&(index, role)| {
                    self.semantics
                        .argument_offset
                        .checked_add(usize::from(index))
                        == Some(body)
                        && role == ArgRole::Body
                }))
            .then_some(grammar);
        }
        Some(grammar)
    }

    /// Literal arguments matching the selected source descriptors' closed
    /// value sets at the actual package floor. Keyword roles take precedence;
    /// option values use the shared prefix/width/reserved-data scan. Unknown
    /// values and excluded options are not enumerable source values.
    #[must_use]
    pub fn authored_source_enum_arguments(&self) -> Vec<usize> {
        // naming.core.original-inlay-retained-context
        // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
        let selected = self.authored_source_descriptors();
        let arguments = self.words.arguments();
        let version = self.semantics.options.availability.package_version;
        let (roles, complete) = self.authored_source_argument_roles();
        if !complete {
            return Vec::new();
        }
        let mut result = Vec::new();
        let mut mark = |argument: usize, values: Vec<&crate::hover::ArgValue>| {
            if roles.iter().any(|&(index, role)| {
                self.semantics
                    .argument_offset
                    .checked_add(usize::from(index))
                    == Some(argument)
                    && role == ArgRole::Keyword
            }) {
                return;
            }
            let matches = arguments.get(argument).is_some_and(|word| match word {
                crate::InvocationWord::Literal(literal) => {
                    values.iter().any(|value| value.value == literal)
                }
                crate::InvocationWord::KnownBytes(bytes) => {
                    values.iter().any(|value| value.value.as_bytes() == bytes)
                }
                _ => false,
            });
            if matches {
                result.push(argument);
            }
        };
        for &(index, _) in selected.command.arg_values {
            mark(
                usize::from(index),
                selected.command.available_arg_values_at(index, version),
            );
        }
        if let Some(subcommand) = selected.subcommand {
            for &(index, _) in subcommand.arg_values {
                if let Some(argument) = self
                    .semantics
                    .argument_offset
                    .checked_add(usize::from(index))
                {
                    mark(argument, subcommand.available_arg_values_at(index, version));
                }
            }
        }
        if let Some(scan) = self.authored_source_diagnostic_options() {
            for option in scan.options.into_iter().filter(|option| option.available) {
                if let Some(values) = option.values {
                    for argument in values {
                        mark(
                            argument,
                            option
                                .option
                                .value_values()
                                .iter()
                                .filter(|value| value.available_for_version(version))
                                .collect(),
                        );
                    }
                }
            }
        }
        result.sort_unstable();
        result.dedup();
        result
    }

    /// Inline case clauses at their original effective ordinals. The selected
    /// case owner validates clauses after the shared option/subject layout;
    /// a computed subject need not be materialised to select static clauses.
    /// Missing literal clause values decline without supplying script entry.
    #[must_use]
    pub fn authored_source_inline_case_clauses(
        &self,
    ) -> Option<Vec<crate::spec::InlineCaseClause>> {
        // naming.source.original-editor-body-structure
        // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md
        let (case, invocation) = self.authored_source_case_invocation()?;
        let start = invocation.inline_clause_start?;
        let arguments = self.words.arguments();
        let count = arguments.exact_argv_len()?;
        let values = (start..count)
            .map(|argument| arguments.literal_at(argument))
            .collect::<Option<Vec<_>>>()?;
        let mut clauses = case.inline_clauses(&values, 0)?;
        for clause in &mut clauses {
            clause.pattern_index = clause.pattern_index.checked_add(start)?;
            clause.body_index = match clause.body_index {
                Some(index) => Some(index.checked_add(start)?),
                None => None,
            };
            for argument in &mut clause.flag_indices {
                *argument = argument.checked_add(start)?;
            }
        }
        Some(clauses)
    }

    fn argument_roles_on_edge(&self, successful: bool) -> (Vec<(u8, ArgRole)>, bool) {
        self.argument_roles_with_native_definition(successful, true)
    }

    fn argument_roles_with_native_definition(
        &self,
        successful: bool,
        native_definition: bool,
    ) -> (Vec<(u8, ArgRole)>, bool) {
        self.argument_roles_with_frame_layout(successful, native_definition, None)
    }

    fn argument_roles_with_frame_layout(
        &self,
        successful: bool,
        native_definition: bool,
        frame_layout: Option<crate::frame_effect::FrameArgumentResolution>,
    ) -> (Vec<(u8, ArgRole)>, bool) {
        let (mut arg_roles, mut arg_roles_complete) = if let Some(effect) =
            self.semantics.frame_effect
            && matches!(
                effect.layout,
                crate::FrameArgLayout::ScriptInSelectedFrame | crate::FrameArgLayout::AliasPairs
            ) {
            let layout = frame_layout.unwrap_or_else(|| {
                if successful {
                    effect.successful_layout(self.words.arguments()).layout
                } else {
                    effect.resolve_arguments(self.words.arguments())
                }
            });
            match layout {
                crate::frame_effect::FrameArgumentResolution::Valid { level_word_len, .. } => {
                    if effect.layout == crate::FrameArgLayout::AliasPairs {
                        let roles = (level_word_len + 1..self.words.arguments().len())
                            .step_by(2)
                            .filter_map(|index| {
                                u8::try_from(index)
                                    .ok()
                                    .map(|index| (index, ArgRole::VarWrite))
                            })
                            .collect();
                        (
                            roles,
                            self.words.arguments().len() <= usize::from(u8::MAX) + 1,
                        )
                    } else {
                        (
                            vec![(
                                u8::try_from(level_word_len)
                                    .expect("optional level has width zero or one"),
                                ArgRole::Body,
                            )],
                            true,
                        )
                    }
                }
                crate::frame_effect::FrameArgumentResolution::Invalid => (Vec::new(), true),
                crate::frame_effect::FrameArgumentResolution::Unknown => (Vec::new(), false),
            }
        } else {
            self.non_frame_argument_roles()
        };
        self.extend_repeated_roles(
            &mut arg_roles,
            &mut arg_roles_complete,
            successful,
            frame_layout,
        );
        match self.semantics.options.value_roles(
            self.words
                .arguments()
                .slice_from(self.semantics.argument_offset),
        ) {
            Some(roles) => {
                for (index, role) in roles {
                    if let Ok(index) = u8::try_from(index) {
                        if !arg_roles.contains(&(index, role)) {
                            arg_roles.push((index, role));
                        }
                    } else {
                        arg_roles_complete = false;
                    }
                }
            }
            None => arg_roles_complete = false,
        }
        if native_definition {
            self.extend_native_definition_roles(&mut arg_roles, &mut arg_roles_complete);
        }
        // Authored optional positions do not name cells when argv proves those
        // arguments absent. Expansion retains the unresolved obligations.
        if let Some(count) = self.words.arguments().exact_argv_len() {
            arg_roles.retain(|(index, _)| {
                self.semantics
                    .argument_offset
                    .checked_add(usize::from(*index))
                    .is_some_and(|index| index < count)
            });
        }
        (arg_roles, arg_roles_complete)
    }

    fn extend_native_definition_roles(
        &self,
        arg_roles: &mut Vec<(u8, ArgRole)>,
        arg_roles_complete: &mut bool,
    ) {
        if let Some(descriptor) = self.semantics.procedure_definition {
            match descriptor.select(self.words.arguments()) {
                crate::native_procedure::NativeProcedureDefinitionSelection::Valid(definition) => {
                    arg_roles.retain(|(_, role)| *role != ArgRole::Body);
                    arg_roles.push((
                        u8::try_from(definition.body_at).expect("native body position"),
                        ArgRole::Body,
                    ));
                }
                crate::native_procedure::NativeProcedureDefinitionSelection::Invalid => {
                    arg_roles.clear();
                }
                crate::native_procedure::NativeProcedureDefinitionSelection::Unknown => {
                    arg_roles.clear();
                    *arg_roles_complete = false;
                }
            }
        }
    }

    /// Materialise the target-neutral facts for an owned consumer such as an
    /// executable IR node.
    ///
    /// Borrowed resolution itself stays allocation-free.  This method is the
    /// explicit ownership boundary: it copies registry identities and
    /// resolves any lazy world-effect descriptors.
    #[must_use]
    pub fn facts(&self) -> InvocationFacts {
        self.facts_on_edge(false)
    }

    /// Materialise facts that hold after successful completion. Keep ordinary
    /// [`Self::facts`] for invocation, error and partial-commit paths.
    #[must_use]
    pub fn facts_after_success(&self) -> InvocationFacts {
        self.facts_on_edge(true)
    }

    fn normal_source_colour(&self) -> Option<crate::taint::TaintColour> {
        use crate::taint::{TaintColour, augment_source_colours};
        let argc = self.words.arguments().exact_argv_len();
        let source = self.semantics.taint_source;
        let traits = self.semantics.traits;
        let selected = source.is_some()
            || traits.intersects(Traits::TAINT_SOURCE | Traits::UNNORMALISED_HTTP_GETTER)
            || (traits.contains(Traits::TAINT_SOURCE_ZERO_ARGS)
                && argc == Some(self.semantics.argument_offset));
        if !selected {
            return None;
        }
        let colour = if argc == Some(0) {
            source.unwrap_or(TaintColour::TAINTED)
        } else {
            TaintColour::TAINTED
        };
        Some(augment_source_colours(colour | TaintColour::TAINTED))
    }

    /// Retain the reached representation-coercion phase independently of
    /// variable writes, result types and mathematical purity. The caller must
    /// still prove the selected native handler before consuming this contract.
    #[must_use]
    pub fn operand_representation_coercions(
        &self,
    ) -> crate::representation::RepresentationCoercionSelection {
        use crate::representation::RepresentationCoercionSelection as Coercions;
        let arguments = self.words.arguments();
        let effect = self
            .semantics
            .representation_effect
            .coercion_selection(arguments, self.semantics.argument_offset);
        if effect != Coercions::None {
            return effect;
        }
        if !self
            .semantics
            .arg_types
            .iter()
            .any(|(_, hint)| hint.shimmers)
        {
            return Coercions::None;
        }
        let Some(hints) = self.selected_argument_type_hints() else {
            return Coercions::Unknown;
        };
        let count = arguments
            .exact_argv_len()
            .expect("selected hints retain exact argc");
        let selected: Vec<_> = self
            .semantics
            .arg_types
            .iter()
            .filter(|(_, hint)| hint.shimmers)
            .filter_map(|(index, _)| hints.first_positional.checked_add(usize::from(*index)))
            .filter(|index| *index < count)
            .collect();
        if selected.is_empty() {
            Coercions::None
        } else {
            Coercions::Operands {
                arguments: selected,
            }
        }
    }

    fn selected_argument_type_hints(&self) -> Option<SelectedArgumentTypeHints> {
        if self.semantics.arg_types.is_empty()
            || matches!(
                self.subcommand,
                SubcommandResolution::Unknown { .. }
                    | SubcommandResolution::Ambiguous { .. }
                    | SubcommandResolution::Indeterminate { .. }
            )
        {
            return None;
        }
        let arguments = self.words.arguments();
        arguments.exact_argv_len()?;
        let offset = self.semantics.argument_offset;
        let skipped = self
            .semantics
            .options
            .leading_word_count(arguments.slice_from(offset))?;
        Some(SelectedArgumentTypeHints {
            table: self.semantics.arg_types,
            first_positional: offset.checked_add(skipped)?,
        })
    }

    fn facts_on_edge(&self, successful: bool) -> InvocationFacts {
        let (transitions, transition_effect_coverage) = if successful {
            self.semantics
                .state_transitions
                .resolve_after_success_with_effect_coverage(self.words.arguments())
        } else {
            self.semantics
                .state_transitions
                .resolve_with_effect_coverage(self.words.arguments())
        };
        let (arg_roles, arg_roles_complete) = self.argument_roles_on_edge(successful);
        let mut facts = InvocationFacts {
            deferred_script_arguments: self.semantics.script_metadata.deferred_arguments(
                self,
                &arg_roles,
                arg_roles_complete,
            ),
            script_lookup_arguments: self.semantics.script_metadata.lookup_arguments(
                self,
                &arg_roles,
                arg_roles_complete,
            ),
            command_prefix_arguments: self.semantics.script_metadata.prefix_arguments(
                self,
                &arg_roles,
                arg_roles_complete,
            ),
            argument_type_hints: self.selected_argument_type_hints(),
            named_object_factory: self.semantics.named_object_factory,
            canonical_command: self.canonical_command.to_owned(),
            subcommand: self.subcommand.into_owned(),
            form: self.form.map(|form| form.name.to_owned()),
            operation: self.semantics.operation,
            lowering_hook: self.semantics.lowering_hook,
            analyser_hook: self.semantics.analyser_hook,
            completion: self.semantics.completion,
            result_stability: self.semantics.result_stability,
            native_result: self.semantics.native_result,
            representation_effect: self.semantics.representation_effect,
            operand_representation_coercions: self.operand_representation_coercions(),
            effects: self.effect_footprint_with_transition_coverage(
                transitions.touches_command_bindings(),
                &transition_effect_coverage,
            ),
            state_transitions: if self.semantics.state_transitions.is_declared() {
                StateTransitionKnowledge::Declared(transitions)
            } else {
                StateTransitionKnowledge::UnknownInvocation
            },
            transition_effect_coverage,
            dispatch_dependencies: self.semantics.dispatch_dependencies.resolve(),
            traits: self.semantics.traits,
            taint_source: self.normal_source_colour(),
            taint_transform: self.semantics.taint_transform.and_then(|transform| {
                transform.resolve(
                    self.words.arguments(),
                    self.semantics.argument_offset,
                    self.semantics.options,
                )
            }),
            mutator: self.semantics.mutator,
            arity: self
                .semantics
                .procedure_definition
                .map_or(self.semantics.arity, |descriptor| {
                    descriptor.arity(self.words.arguments().dialect())
                }),
            arity_argument_count: self.argument_count_for_arity(),
            frozen_argument_count: self.words.arguments().exact_argv_len(),
            argument_offset: self.semantics.argument_offset,
            arg_roles,
            repeated_args: self.semantics.repeated_args.to_vec(),
            arg_roles_complete,
            arg_role_resolver_roles: self.semantics.arg_role_resolver_roles,
            return_type: self.semantics.return_type,
            procedure_definition: self
                .semantics
                .procedure_definition
                .map(|descriptor| descriptor.select(self.words.arguments())),
            native_compilation: self.semantics.native_compilation,
            successful_handler: self.semantics.successful_handler,
            variable_receivers: self.semantics.variable_receivers,
            byte_array_effect: self.semantics.byte_array_effect,
            byte_array_payload: self.semantics.byte_array_payload,
            var_write_typing: self.semantics.var_write_typing,
            return_elements: self.semantics.return_elements,
            var_elements_effect: self.semantics.var_elements_effect,
            body_kind: self.semantics.body_kind,
            body_execution: self.semantics.body_execution,
            body_interpreter: self.semantics.body_interpreter,
            frame_effect: self.semantics.frame_effect,
        };
        if successful
            && let Some(effects) = facts.successful_value_leaf_world(self.words.arguments())
        {
            facts.effects = effects;
            facts.state_transitions =
                StateTransitionKnowledge::Declared(StateTransitions::default());
        }
        facts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::InvocationArguments;
    use crate::world_effect::{
        CallbackKinds, EffectAccess, EffectAccessMode, EffectFootprint, StaticEffectAccess,
        StaticEffectFootprint, StaticInterpreterScope, StaticNamespaceScope, StaticSubjectScope,
        SubjectScope, WorldEffectComposition, WorldEffectDescriptor, WorldStateDomain,
    };
    use crate::{
        AbruptTransitionTransfer, CallerFrameSelection, CommandBindingDefinitionKind,
        CommandBindingTransition, CommandRegistry, CompletionCode, CompletionCodeDomain,
        CompletionDescriptor, DispatchDependencies, DispatchDependencyDescriptor,
        DispatchDependencyDomain, NamespaceTransition, NamespaceTransitionTarget, Reentrancy,
        StateTransition, StateTransitionCommit, StateTransitionDescriptor, StateTransitionDomain,
        StateTransitionKnowledge, TransitionSubject, VariableAliasTarget,
    };
    use tcl_dialect::model::{Family, SpecSurface, SurfaceQuery};

    #[test]
    fn source_enum_projection_keeps_literal_and_native_byte_facets_separate() {
        // naming.core.original-inlay-retained-context
        // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
        let mut registry = CommandRegistry::build_default();
        registry.insert(crate::CommandSpec {
            name: "enum_source_test",
            arg_values: &[(
                0,
                &[crate::hover::ArgValue {
                    value: "ready",
                    ..crate::hover::ArgValue::DEFAULT
                }],
            )],
            ..crate::CommandSpec::DEFAULT
        });
        for (value, expected) in [
            (crate::InvocationWord::Literal("ready"), vec![0]),
            (crate::InvocationWord::KnownBytes(b"ready"), vec![0]),
            (crate::InvocationWord::Literal("other"), vec![]),
            (crate::InvocationWord::KnownBytes(b"ready\0tail"), vec![]),
            (crate::InvocationWord::Dynamic, vec![]),
            (crate::InvocationWord::Expanded, vec![]),
            (crate::InvocationWord::Opaque, vec![]),
        ] {
            let arguments = [value];
            let selected = registry.resolve_structured_invocation(
                crate::InvocationWords::structured(
                    crate::InvocationWord::Literal("enum_source_test"),
                    &arguments,
                ),
                None,
            );
            assert_eq!(
                selected
                    .resolved()
                    .unwrap()
                    .authored_source_enum_arguments(),
                expected
            );
        }
    }

    #[test]
    fn source_possible_options_branch_over_admitted_widths_and_terminators() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // Pure selected descriptor/source layout, without handler execution.
        static OPTIONS: &[crate::hover::OptionSpec] = &[
            crate::hover::OptionSpec {
                name: "--",
                surface: Some(SpecSurface::TCL86_PLUS),
                ..crate::hover::OptionSpec::DEFAULT
            },
            crate::hover::OptionSpec {
                name: "-flag",
                ..crate::hover::OptionSpec::DEFAULT
            },
            crate::hover::OptionSpec {
                name: "-value",
                value: crate::hover::OptionValue::value("value"),
                ..crate::hover::OptionSpec::DEFAULT
            },
        ];
        let mut registry = CommandRegistry::build_default();
        registry.insert(crate::CommandSpec {
            name: "source-options",
            options: OPTIONS,
            ..crate::CommandSpec::DEFAULT
        });
        let dynamic = crate::InvocationWord::Dynamic;
        let literal = crate::InvocationWord::Literal;
        for (arguments, expected) in [
            (vec![dynamic, literal("data"), dynamic], vec![0, 2]),
            (vec![literal("-v"), dynamic, dynamic], vec![0, 2]),
            (vec![literal("--"), dynamic], vec![]),
            (vec![literal("data"), dynamic], vec![]),
            (vec![literal("-missing"), dynamic], vec![0]),
        ] {
            let words = crate::InvocationWords::structured(literal("source-options"), &arguments);
            let selected = registry
                .resolve_structured_invocation(words, Some(SurfaceQuery::core(Family::Tcl, "8.6")))
                .resolved()
                .unwrap();
            assert_eq!(
                selected
                    .authored_source_possible_option_arguments()
                    .unwrap()
                    .arguments,
                expected
            );
            let older = registry
                .resolve_structured_invocation(words, Some(SurfaceQuery::core(Family::Tcl, "8.4")))
                .resolved()
                .unwrap();
            assert!(older.authored_source_possible_option_arguments().is_none());
        }
        let expanded = [crate::InvocationWord::Expanded];
        let selected = registry
            .resolve_structured_invocation(
                crate::InvocationWords::structured(literal("source-options"), &expanded),
                Some(SurfaceQuery::core(Family::Tcl, "8.6")),
            )
            .resolved()
            .unwrap();
        assert!(
            selected
                .authored_source_possible_option_arguments()
                .is_none()
        );
    }

    #[test]
    fn selected_option_variable_scope_shares_availability_prefix_and_value_geometry() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        static OPTIONS: &[crate::hover::OptionSpec] = &[
            crate::hover::OptionSpec {
                name: "-global",
                value: crate::hover::OptionValue::global_var_name(),
                surface: Some(tcl_dialect::model::SpecSurface::TCL86_PLUS),
                ..crate::hover::OptionSpec::DEFAULT
            },
            crate::hover::OptionSpec {
                name: "-local",
                value: crate::hover::OptionValue::var_name(),
                ..crate::hover::OptionSpec::DEFAULT
            },
        ];
        let mut registry = CommandRegistry::build_default();
        registry.insert(crate::CommandSpec {
            name: "scope-owner",
            options: OPTIONS,
            arity: crate::Arity::new(0, 4),
            ..crate::CommandSpec::DEFAULT
        });
        let query = Some(SurfaceQuery::core(Family::Tcl, "8.6"));
        let values = [
            crate::InvocationWord::Literal("-g"),
            crate::InvocationWord::Dynamic,
            crate::InvocationWord::Literal("-local"),
            crate::InvocationWord::Literal("named"),
        ];
        let selected = registry
            .resolve_structured_invocation(
                crate::InvocationWords::structured(
                    crate::InvocationWord::Literal("scope-owner"),
                    &values,
                ),
                query,
            )
            .resolved()
            .unwrap();
        assert_eq!(
            selected.authored_source_option_variable_scope_at(1),
            Some(crate::VariableScope::Global)
        );
        assert_eq!(
            selected.authored_source_option_variable_scope_at(3),
            Some(crate::VariableScope::CurrentFrame)
        );
        assert!(
            selected
                .authored_source_option_variable_scope_at(0)
                .is_none()
        );
        let old = registry
            .resolve_structured_invocation(
                crate::InvocationWords::structured(
                    crate::InvocationWord::Literal("scope-owner"),
                    &values,
                ),
                Some(SurfaceQuery::core(Family::Tcl, "8.4")),
            )
            .resolved()
            .unwrap();
        assert!(old.authored_source_option_variable_scope_at(1).is_none());
        let dynamic = [
            crate::InvocationWord::Dynamic,
            crate::InvocationWord::Literal("named"),
        ];
        let unknown = registry
            .resolve_structured_invocation(
                crate::InvocationWords::structured(
                    crate::InvocationWord::Literal("scope-owner"),
                    &dynamic,
                ),
                query,
            )
            .resolved()
            .unwrap();
        assert!(
            unknown
                .authored_source_option_variable_scope_at(1)
                .is_none()
        );
        let terminated = registry
            .resolve_invocation("scope-owner", &["--", "-global", "named"], query)
            .unwrap();
        assert!(
            terminated
                .authored_source_option_variable_scope_at(2)
                .is_none()
        );
        assert_eq!(
            registry.option_variable_scope("scope-owner", &["-g", "named"], 1, query),
            Some(crate::VariableScope::Global)
        );
    }

    #[test]
    fn source_enum_values_and_definition_bodies_use_the_selected_invocation() {
        // naming.core.original-inlay-retained-context
        // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
        let registry = CommandRegistry::build_default();
        let dialect = crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_1);
        let query = Some(SurfaceQuery::core(Family::Tcl, "9.1"));
        let args = [
            crate::InvocationWord::Literal("is"),
            crate::InvocationWord::Literal("alnum"),
            crate::InvocationWord::Dynamic,
        ];
        let selected = registry
            .resolve_structured_invocation(
                crate::InvocationWords::structured(crate::InvocationWord::Literal("string"), &args)
                    .with_dialect(dialect),
                query,
            )
            .resolved()
            .unwrap();
        assert_eq!(selected.authored_source_enum_arguments(), vec![1]);
        for (args, definition_body) in [
            (vec!["C", "method pick {} {}"], true),
            (vec!["C", "method", "pick", "{}", "{}"], false),
        ] {
            let words = args
                .iter()
                .map(|word| crate::InvocationWord::Literal(word))
                .collect::<Vec<_>>();
            let selected = registry
                .resolve_structured_invocation(
                    crate::InvocationWords::structured(
                        crate::InvocationWord::Literal("oo::define"),
                        &words,
                    )
                    .with_dialect(dialect),
                    query,
                )
                .resolved()
                .unwrap();
            assert_eq!(
                selected.authored_source_definition_body_grammar().is_some(),
                definition_body,
                "{args:?}"
            );
        }
        let args = [
            crate::InvocationWord::Dynamic,
            crate::InvocationWord::Literal("one"),
            crate::InvocationWord::Literal("set x 1"),
            crate::InvocationWord::Literal("default"),
            crate::InvocationWord::Literal("set x 2"),
        ];
        let selected = registry
            .resolve_structured_invocation(
                crate::InvocationWords::structured(crate::InvocationWord::Literal("switch"), &args)
                    .with_dialect(dialect),
                query,
            )
            .resolved()
            .unwrap();
        let clauses = selected.authored_source_inline_case_clauses().unwrap();
        assert_eq!(clauses.len(), 2);
        assert_eq!(clauses[0].pattern_index, 1);
        assert_eq!(clauses[0].body_index, Some(2));
        assert_eq!(clauses[1].body_index, Some(4));
    }

    #[test]
    fn source_diagnostic_options_keep_nested_prefixes_and_unknown_boundaries() {
        // naming.diagnostic.registry-source-ownership
        // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
        let registry = crate::CommandRegistry::build_default();
        let dialect = crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
        for (words, expected) in [
            (vec!["ensemble", "create", "-command", "::E"], 2),
            (vec!["ensemble", "configure", "-map", "-prefixes", "0"], 3),
        ] {
            let args = words
                .iter()
                .map(|word| crate::InvocationWord::Literal(word))
                .collect::<Vec<_>>();
            let invocation = registry
                .resolve_structured_invocation(
                    crate::InvocationWords::structured(
                        crate::InvocationWord::Literal("namespace"),
                        &args,
                    )
                    .with_dialect(dialect),
                    Some(tcl_dialect::model::SurfaceQuery::core(
                        tcl_dialect::model::Family::Tcl,
                        "8.6",
                    )),
                )
                .resolved()
                .unwrap();
            let scan = invocation.authored_source_diagnostic_options().unwrap();
            assert_eq!(scan.options.len(), 1, "{words:?}: {scan:?}");
            assert_eq!(scan.options[0].argument, expected);
        }
        let args = [
            crate::InvocationWord::Literal("-nocase"),
            crate::InvocationWord::Dynamic,
            crate::InvocationWord::Literal("-all"),
        ];
        let invocation = registry
            .resolve_structured_invocation(
                crate::InvocationWords::structured(crate::InvocationWord::Literal("regexp"), &args)
                    .with_dialect(dialect),
                Some(tcl_dialect::model::SurfaceQuery::core(
                    tcl_dialect::model::Family::Tcl,
                    "8.6",
                )),
            )
            .resolved()
            .unwrap();
        let scan = invocation.authored_source_diagnostic_options().unwrap();
        assert_eq!(scan.options.len(), 1);
        assert_eq!(
            scan.boundary,
            super::AuthoredSourceOptionBoundary::Dynamic(1)
        );
    }

    #[test]
    fn authored_source_procedure_roles_do_not_supply_native_definition_acceptance() {
        // Implementation contract: naming.vendor.original-registry-metadata
        // docs/design/analysis/name-resolution-proofs/vendor-original-registry-metadata.md
        let registry = CommandRegistry::build_default();
        let arguments = [
            crate::InvocationWord::Literal("p"),
            crate::InvocationWord::Literal("arg"),
            crate::InvocationWord::Opaque,
        ];
        let selected = registry
            .resolve_structured_invocation(
                crate::InvocationWords::structured(
                    crate::InvocationWord::Literal("proc"),
                    &arguments,
                ),
                None,
            )
            .resolved()
            .unwrap();
        let (roles, complete) = selected.authored_source_argument_roles();
        assert!(complete);
        assert!(roles.contains(&(1, ArgRole::ParamList)));
        assert!(roles.contains(&(2, ArgRole::Body)));
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        assert_eq!(
            selected.authored_source_procedure_arguments(),
            Some(AuthoredSourceProcedureArguments {
                name: 0,
                parameters: 1,
                body: 2,
            })
        );
        for facts in [selected.facts(), selected.facts_after_success()] {
            assert!(!facts.arg_roles_complete);
            assert!(facts.arg_roles.is_empty());
        }
        let expanded = [crate::InvocationWord::Expanded];
        let selected = registry
            .resolve_structured_invocation(
                crate::InvocationWords::structured(
                    crate::InvocationWord::Literal("proc"),
                    &expanded,
                ),
                None,
            )
            .resolved()
            .unwrap();
        assert!(selected.words.arguments().exact_argv_len().is_none());
        assert!(selected.authored_source_procedure_arguments().is_none());
        assert!(!selected.facts().arg_roles_complete);
    }

    #[test]
    fn selected_subcommand_options_preserve_reserved_positional_values() {
        // naming.minifier.complete-logical-metadata
        // docs/design/analysis/name-resolution-proofs/minifier-complete-logical-metadata.md
        let registry = crate::CommandRegistry::build_default();
        let dialect = crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
        for (arguments, expected) in [
            (vec!["equal", "-nocase", "-nocase", "other"], 1),
            (vec!["compare", "-nocase", "-nocase", "other"], 1),
            (vec!["equal", "-nocase", "-nocase", "first", "other"], 2),
            (vec!["equal", "-length", "2", "-nocase", "other"], 2),
            (vec!["equal", "first", "-nocase"], 0),
        ] {
            let words = arguments
                .iter()
                .map(|value| crate::InvocationWord::Literal(value))
                .collect::<Vec<_>>();
            let selected = registry
                .resolve_structured_invocation(
                    crate::InvocationWords::structured(
                        crate::InvocationWord::Literal("string"),
                        &words,
                    )
                    .with_dialect(dialect),
                    dialect.authoring_query(),
                )
                .resolved()
                .unwrap();
            assert_eq!(selected.semantics.options.reserved_trailing_words, 2);
            assert_eq!(
                selected
                    .semantics
                    .options
                    .leading_word_count(selected.words.arguments().slice_from(1)),
                Some(expected),
                "{arguments:?}"
            );
        }
    }

    #[test]
    fn authored_case_presentation_keeps_incomplete_list_separate_from_invocation() {
        // Implementation contract: naming.source.original-editor-body-structure
        // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md
        let registry = crate::CommandRegistry::build_default();
        for version in tcl_dialect::TclVersion::ALL {
            let words = [
                crate::InvocationWord::Literal("--"),
                crate::InvocationWord::Dynamic,
                crate::InvocationWord::Literal("x {puts x} dangling"),
            ];
            let selected = registry
                .resolve_structured_invocation(
                    crate::InvocationWords::structured(
                        crate::InvocationWord::Literal("switch"),
                        &words,
                    )
                    .with_dialect(crate::InvocationDialect::for_version(version)),
                    crate::InvocationDialect::for_version(version).authoring_query(),
                )
                .resolved()
                .unwrap();
            assert!(selected.authored_source_case_invocation().is_none());
            assert_eq!(
                selected
                    .authored_source_case_presentation()
                    .map(|(_, argument)| argument),
                Some(2)
            );
            for list in ["", "x {puts x}", "x {puts x"] {
                let words = [
                    crate::InvocationWord::Literal("--"),
                    crate::InvocationWord::Dynamic,
                    crate::InvocationWord::Literal(list),
                ];
                let selected = registry
                    .resolve_structured_invocation(
                        crate::InvocationWords::structured(
                            crate::InvocationWord::Literal("switch"),
                            &words,
                        )
                        .with_dialect(crate::InvocationDialect::for_version(version)),
                        crate::InvocationDialect::for_version(version).authoring_query(),
                    )
                    .resolved()
                    .unwrap();
                assert!(selected.authored_source_case_presentation().is_none());
            }
            let words = [
                crate::InvocationWord::Literal("--"),
                crate::InvocationWord::Dynamic,
                crate::InvocationWord::Expanded,
                crate::InvocationWord::Literal("x {puts x} dangling"),
            ];
            let selected = registry
                .resolve_structured_invocation(
                    crate::InvocationWords::structured(
                        crate::InvocationWord::Literal("switch"),
                        &words,
                    )
                    .with_dialect(crate::InvocationDialect::for_version(version)),
                    crate::InvocationDialect::for_version(version).authoring_query(),
                )
                .resolved()
                .unwrap();
            assert!(selected.authored_source_case_presentation().is_none());
        }
    }

    #[test]
    fn authored_case_layout_retains_dynamic_subject_and_exact_control_words() {
        // Implementation contract: naming.source.original-editor-body-structure
        // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md
        let registry = CommandRegistry::build_default();
        for version in tcl_dialect::TclVersion::ALL {
            let dialect = crate::InvocationDialect::for_version(version);
            let arguments = [
                crate::InvocationWord::KnownBytes(b"--"),
                crate::InvocationWord::Dynamic,
                crate::InvocationWord::KnownBytes(b"x {puts x} default {puts fallback}"),
            ];
            let selected = registry
                .resolve_structured_invocation(
                    crate::InvocationWords::structured(
                        crate::InvocationWord::Literal("switch"),
                        &arguments,
                    )
                    .with_dialect(dialect),
                    dialect.authoring_query(),
                )
                .resolved()
                .unwrap();
            let (_, layout) = selected.authored_source_case_invocation().unwrap();
            assert_eq!(layout.subject_index, Some(1));
            assert_eq!(layout.clause_list_index, Some(2));
            assert!(selected.words.arguments().literal_at(1).is_none());
            let expanded = [crate::InvocationWord::Expanded];
            let selected = registry
                .resolve_structured_invocation(
                    crate::InvocationWords::structured(
                        crate::InvocationWord::Literal("switch"),
                        &expanded,
                    )
                    .with_dialect(dialect),
                    dialect.authoring_query(),
                )
                .resolved()
                .unwrap();
            assert!(selected.authored_source_case_invocation().is_none());
        }
    }

    #[test]
    fn selected_option_value_roles_reach_owned_facts_without_classifying_result_data() {
        let registry = crate::CommandRegistry::build_default();
        for arguments in [
            vec!["name", "description", "-body", "expr $x+1", "-result", ""],
            vec!["name", "description", "-result", "-body"],
        ] {
            let resolved = registry
                .resolve_invocation("tcltest::test", &arguments, None)
                .expect("available declared option layout");
            let expected = (arguments.len() == 6).then_some((3, ArgRole::Body));
            for facts in [resolved.facts(), resolved.facts_after_success()] {
                let bodies: Vec<_> = facts
                    .arg_roles
                    .into_iter()
                    .filter(|(_, role)| *role == ArgRole::Body)
                    .collect();
                assert_eq!(bodies, expected.into_iter().collect::<Vec<_>>());
            }
        }
    }

    #[test]
    fn selected_option_value_role_width_is_independent_of_unknown_payload() {
        use crate::InvocationWord::{Dynamic, Literal};
        let registry = CommandRegistry::build_default();
        let arguments = [
            Literal("name"),
            Literal("description"),
            Literal("-body"),
            Dynamic,
        ];
        let resolved = registry
            .resolve_structured_invocation(
                crate::InvocationWords::structured(Literal("tcltest::test"), &arguments),
                None,
            )
            .resolved()
            .expect("selected package layout");
        assert!(resolved.facts().arg_roles.contains(&(3, ArgRole::Body)));
        let unknown = [
            Literal("name"),
            Literal("description"),
            Dynamic,
            Literal("expr $x"),
        ];
        let resolved = registry
            .resolve_structured_invocation(
                crate::InvocationWords::structured(Literal("tcltest::test"), &unknown),
                None,
            )
            .resolved()
            .expect("unresolved option word remains an invocation");
        assert!(!resolved.facts().arg_roles_complete);
        assert!(!resolved.facts().arg_roles.contains(&(3, ArgRole::Body)));
    }

    #[test]
    fn selected_operand_coercions_preserve_shared_object_hazards() {
        use crate::InvocationWord::{Dynamic, Expanded, Literal};
        use crate::representation::RepresentationCoercionSelection as Coercions;
        let context = crate::model::ingress::static_context_for("tcl9.0");
        let registry = context.commands();
        let dialect = crate::InvocationDialect::of_profile(registry.profile().unwrap());
        let cases: &[(&str, &[crate::InvocationWord<'_>], Coercions)] = &[
            (
                "llength",
                &[Dynamic],
                Coercions::Operands { arguments: vec![0] },
            ),
            ("llength", &[Expanded], Coercions::Unknown),
            ("list", &[Dynamic], Coercions::None),
            (
                "string",
                &[Literal("length"), Dynamic],
                Coercions::Operands { arguments: vec![1] },
            ),
            (
                "expr",
                &[Literal("2 + 2")],
                Coercions::Expression {
                    source_arguments: 0..1,
                },
            ),
        ];
        for (head, arguments, expected) in cases {
            let words =
                crate::InvocationWords::structured(Literal(head), arguments).with_dialect(dialect);
            let invocation = registry
                .resolve_structured_invocation(words, None)
                .resolved()
                .unwrap();
            assert_eq!(
                invocation.operand_representation_coercions(),
                *expected,
                "{head}"
            );
            assert_eq!(
                invocation.facts().operand_representation_coercions,
                *expected,
                "{head}"
            );
        }
    }

    #[test]
    fn selected_regexp_result_uses_available_flags_before_unknown_values() {
        use crate::InvocationWord::{Dynamic, Literal};
        for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let profile = crate::model::ingress::static_context_for(environment)
                .commands()
                .profile()
                .unwrap();
            // The raw catalogue's latest row is Jim; actual operand ingress
            // must select the C row independently of that insertion order.
            let registry = crate::CommandRegistry::build_default();
            let dialect = crate::InvocationDialect::of_profile(profile);
            for (words, expected) in [
                (
                    vec![Literal("-about"), Dynamic],
                    (environment != "jim").then_some(crate::TclType::List),
                ),
                (
                    vec![Literal("--"), Literal("-about"), Dynamic],
                    Some(crate::TclType::Int),
                ),
                (vec![Dynamic, Dynamic], None),
                (vec![Literal("-inline"), Dynamic, Dynamic], None),
                (
                    vec![Literal("-start"), Literal("-about"), Dynamic, Dynamic],
                    None,
                ),
                (
                    vec![Literal("-start"), Literal("--"), Dynamic, Dynamic],
                    None,
                ),
            ] {
                let facts = registry
                    .resolve_structured_invocation(
                        crate::InvocationWords::structured(Literal("regexp"), &words)
                            .with_dialect(dialect),
                        None,
                    )
                    .resolved()
                    .unwrap()
                    .facts();
                assert_eq!(facts.return_type, expected, "{environment}: {words:?}");
            }
        }
    }

    #[test]
    fn authored_source_shape_advice_keeps_unknown_payloads_and_selected_dialect() {
        // naming.diagnostic.registry-source-ownership
        // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
        use crate::InvocationWord::{Dynamic, Expanded, Literal};
        for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let context =
                crate::model::ingress::resolve_environment(environment).default_context_registry();
            let dialect =
                crate::InvocationDialect::of_profile(context.commands().profile().unwrap());
            for arguments in [
                vec![Dynamic, Dynamic, Dynamic],
                vec![Literal("-regexp"), Dynamic, Dynamic, Dynamic],
            ] {
                let selected =
                    crate::model::assembly::resolve_structured_invocation_in_resolved_context(
                        context.commands(),
                        Some(context.context()),
                        crate::InvocationWords::structured(Literal("switch"), &arguments)
                            .with_dialect(dialect),
                        tcl_dialect::model::InvocationRealm::RuleLoader,
                    )
                    .resolved()
                    .unwrap();
                let bodies = selected.authored_source_case_body_arguments().unwrap();
                assert_eq!(bodies.len(), 1, "{environment}");
                assert_eq!(bodies[0].argument, arguments.len() - 1);
                assert_eq!(bodies[0].regexp, arguments.len() == 4);
            }
            for (arguments, expected) in [
                (vec![Literal("-nocomplain")], true),
                (vec![Literal("--")], true),
                (vec![Literal("-nocomplain"), Literal("--")], true),
                (
                    vec![Literal("-nocomplain"), Literal("-nocomplain")],
                    environment == "jim",
                ),
                (vec![Literal("-nocomplain"), Dynamic], false),
                (vec![Expanded], false),
            ] {
                let selected =
                    crate::model::assembly::resolve_structured_invocation_in_resolved_context(
                        context.commands(),
                        Some(context.context()),
                        crate::InvocationWords::structured(Literal("unset"), &arguments)
                            .with_dialect(dialect),
                        tcl_dialect::model::InvocationRealm::RuleLoader,
                    )
                    .resolved()
                    .unwrap();
                assert_eq!(
                    selected
                        .authored_source_unset_option_only_arguments()
                        .is_some(),
                    expected,
                    "{environment}: {arguments:?}"
                );
            }
            let arguments = [Dynamic, Dynamic];
            let selected =
                crate::model::assembly::resolve_structured_invocation_in_resolved_context(
                    context.commands(),
                    Some(context.context()),
                    crate::InvocationWords::structured(Literal("append"), &arguments)
                        .with_dialect(dialect),
                    tcl_dialect::model::InvocationRealm::RuleLoader,
                )
                .resolved()
                .unwrap();
            let append = selected.authored_source_append_arguments().unwrap();
            assert_eq!(append.variable, 0);
            assert_eq!(append.values, 1..2);
            let arguments = [Dynamic, Expanded];
            let selected =
                crate::model::assembly::resolve_structured_invocation_in_resolved_context(
                    context.commands(),
                    Some(context.context()),
                    crate::InvocationWords::structured(Literal("append"), &arguments)
                        .with_dialect(dialect),
                    tcl_dialect::model::InvocationRealm::RuleLoader,
                )
                .resolved()
                .unwrap();
            assert!(selected.authored_source_append_arguments().is_none());
        }
    }

    #[test]
    fn authored_source_patterns_keep_selected_context_and_option_languages() {
        // naming.core.original-pattern-retained-context
        // docs/design/analysis/name-resolution-proofs/original-pattern-retained-context.md
        use crate::InvocationWord::{Dynamic, Literal};
        let driver =
            crate::model::ingress::resolve_environment("tcl9.1").default_context_registry();
        let older = crate::model::ingress::resolve_environment("tcl8.4").default_context_registry();
        let older = older.with_command_store(driver.commands().snapshot().shared_registry());
        let dialect = crate::InvocationDialect::of_profile(driver.commands().profile().unwrap());
        let arguments = [
            Literal("-regexp"),
            Literal("-stride"),
            Literal("2"),
            Dynamic,
            Literal("a+"),
        ];
        for (context, expected) in [(&*driver, Some(4)), (&older, None)] {
            let selected =
                crate::model::assembly::resolve_structured_invocation_in_resolved_context(
                    context.commands(),
                    Some(context.context()),
                    crate::InvocationWords::structured(Literal("lsearch"), &arguments)
                        .with_dialect(dialect),
                    tcl_dialect::model::InvocationRealm::RuleLoader,
                )
                .resolved()
                .unwrap();
            let patterns = selected.authored_source_pattern_arguments();
            assert_eq!(
                patterns
                    .as_ref()
                    .and_then(|patterns| patterns.first().map(|pattern| pattern.index)),
                expected
            );
            if expected.is_some() {
                assert_eq!(
                    patterns.unwrap(),
                    vec![crate::patterns::PatternArg {
                        index: 4,
                        kind: crate::patterns::PatternType::Regex
                    }]
                );
            } else {
                assert!(patterns.is_none());
            }
        }
        for (selector, expected) in [
            ("-glob", Some(crate::patterns::PatternType::Glob)),
            ("-regexp", Some(crate::patterns::PatternType::Regex)),
            ("-exact", None),
            ("-sorted", None),
        ] {
            let arguments = [Literal(selector), Dynamic, Literal("a+")];
            let selected =
                crate::model::assembly::resolve_structured_invocation_in_resolved_context(
                    driver.commands(),
                    Some(driver.context()),
                    crate::InvocationWords::structured(Literal("lsearch"), &arguments)
                        .with_dialect(dialect),
                    tcl_dialect::model::InvocationRealm::RuleLoader,
                )
                .resolved()
                .unwrap();
            let patterns = selected.authored_source_pattern_arguments().unwrap();
            assert_eq!(
                patterns
                    .first()
                    .map(|pattern| (pattern.index, pattern.kind)),
                expected.map(|kind| (2, kind))
            );
        }
    }

    #[test]
    fn authored_source_patterns_keep_payload_unknown_and_refuse_uncertain_layout() {
        // naming.core.original-pattern-retained-context
        // docs/design/analysis/name-resolution-proofs/original-pattern-retained-context.md
        use crate::InvocationWord::{Dynamic, Expanded, Literal};
        use crate::patterns::PatternType::{Glob, Regex};
        let context =
            crate::model::ingress::resolve_environment("tcl9.1").default_context_registry();
        let dialect = crate::InvocationDialect::of_profile(context.commands().profile().unwrap());
        for (head, arguments, expected) in [
            (
                "regexp",
                vec![Literal("-start"), Literal("0"), Literal("a+"), Dynamic],
                vec![(2, Regex)],
            ),
            (
                "regsub",
                vec![
                    Literal("-all"),
                    Literal("a+"),
                    Dynamic,
                    Literal("replacement"),
                ],
                vec![(1, Regex)],
            ),
            (
                "string",
                vec![Literal("match"), Literal("-nocase"), Literal("a*"), Dynamic],
                vec![(2, Glob)],
            ),
            (
                "glob",
                vec![Literal("-directory"), Dynamic, Literal("a*"), Dynamic],
                vec![(2, Glob), (3, Glob)],
            ),
        ] {
            let selected =
                crate::model::assembly::resolve_structured_invocation_in_resolved_context(
                    context.commands(),
                    Some(context.context()),
                    crate::InvocationWords::structured(Literal(head), &arguments)
                        .with_dialect(dialect),
                    tcl_dialect::model::InvocationRealm::RuleLoader,
                )
                .resolved()
                .unwrap();
            let patterns = selected.authored_source_pattern_arguments().unwrap();
            assert_eq!(
                patterns
                    .iter()
                    .map(|pattern| (pattern.index, pattern.kind))
                    .collect::<Vec<_>>(),
                expected,
                "{head}"
            );
            assert!(selected.words.arguments().literal_values().is_none());
        }
        for (head, arguments) in [
            ("regexp", vec![Dynamic, Literal("a+"), Dynamic]),
            ("lsearch", vec![Dynamic, Dynamic, Literal("a+")]),
            ("glob", vec![Dynamic, Literal("a*")]),
            ("string", vec![Dynamic, Literal("a*"), Dynamic]),
            ("lsearch", vec![Literal("-regexp"), Expanded, Literal("a+")]),
        ] {
            let selected =
                crate::model::assembly::resolve_structured_invocation_in_resolved_context(
                    context.commands(),
                    Some(context.context()),
                    crate::InvocationWords::structured(Literal(head), &arguments)
                        .with_dialect(dialect),
                    tcl_dialect::model::InvocationRealm::RuleLoader,
                )
                .resolved()
                .unwrap();
            assert!(
                selected.authored_source_pattern_arguments().is_none(),
                "{head}: {arguments:?}"
            );
        }
    }

    #[test]
    fn authored_source_formats_keep_selected_context_and_effective_ordinals() {
        // naming.core.original-inlay-retained-context
        // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
        use crate::InvocationWord::{Dynamic, Literal};
        let driver =
            crate::model::ingress::resolve_environment("tcl9.1").default_context_registry();
        let older = crate::model::ingress::resolve_environment("tcl8.4").default_context_registry();
        let older = older.with_command_store(driver.commands().snapshot().shared_registry());
        let dialect = crate::InvocationDialect::of_profile(driver.commands().profile().unwrap());
        let arguments = [
            Literal("scan"),
            Literal("2020"),
            Literal("-format"),
            Literal("%Y"),
        ];
        for (context, expected) in [(&*driver, Some(3)), (&older, None)] {
            let selected =
                crate::model::assembly::resolve_structured_invocation_in_resolved_context(
                    context.commands(),
                    Some(context.context()),
                    crate::InvocationWords::structured(Literal("clock"), &arguments)
                        .with_dialect(dialect),
                    tcl_dialect::model::InvocationRealm::RuleLoader,
                )
                .resolved()
                .unwrap();
            let formats = selected.authored_source_format_arguments();
            assert_eq!(
                formats
                    .as_ref()
                    .and_then(|formats| formats.first().map(|format| format.index)),
                expected
            );
            if expected.is_some() {
                let formats = formats.unwrap();
                assert_eq!(formats.len(), 1);
                assert_eq!(formats[0].kind, crate::FormatType::Clock);
            } else {
                assert!(formats.is_none_or(|formats| formats.is_empty()));
            }
        }
        for (head, arguments, index, kind) in [
            (
                "format",
                vec![Literal("%d"), Dynamic],
                0,
                crate::FormatType::Sprintf,
            ),
            (
                "binary",
                vec![Literal("format"), Literal("c"), Dynamic],
                1,
                crate::FormatType::Binary,
            ),
        ] {
            let selected =
                crate::model::assembly::resolve_structured_invocation_in_resolved_context(
                    driver.commands(),
                    Some(driver.context()),
                    crate::InvocationWords::structured(Literal(head), &arguments)
                        .with_dialect(dialect),
                    tcl_dialect::model::InvocationRealm::RuleLoader,
                )
                .resolved()
                .unwrap();
            let formats = selected.authored_source_format_arguments().unwrap();
            assert_eq!(formats.len(), 1);
            assert_eq!(formats[0].index, index);
            assert_eq!(formats[0].kind, kind);
            assert!(!formats[0].scan);
        }
    }

    #[test]
    fn source_arity_count_layout_keeps_axis_prefix_data_and_expansion_bounds() {
        // naming.diagnostic.registry-source-ownership
        // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
        use crate::InvocationWord::{Dynamic, Expanded, Literal};
        let owner = crate::model::ingress::static_context_for("tcl8.6");
        for (head, arguments, minimum, indeterminate, operands) in [
            (
                "set",
                vec![Literal("name"), Dynamic, Expanded, Literal("extra")],
                3,
                true,
                vec![0, 1, 3],
            ),
            ("string", vec![Literal("le"), Dynamic], 1, false, vec![1]),
            ("fconfigure", vec![], 0, false, vec![]),
            ("fconfigure", vec![Dynamic], 1, false, vec![0]),
        ] {
            let resolution = owner.commands().resolve_structured_invocation(
                crate::InvocationWords::structured(Literal(head), &arguments),
                Some(owner.context().authoring_query()),
            );
            let selected = resolution.resolved().unwrap();
            let count = selected.authored_source_arity().unwrap().count;
            assert_eq!(
                (count.minimum, count.indeterminate, count.operands),
                (minimum, indeterminate, operands),
                "{head}"
            );
        }
        let arguments = [Dynamic, Literal("-buffering"), Literal("none")];
        let resolution = owner.commands().resolve_structured_invocation(
            crate::InvocationWords::structured(Literal("fconfigure"), &arguments),
            Some(owner.context().authoring_query()),
        );
        let selected = resolution.resolved().unwrap();
        let all = selected
            .authored_source_count_for_arity(crate::Arity::any())
            .unwrap();
        assert_eq!(all.operands, vec![0, 1, 2]);
        let positional = selected
            .authored_source_count_for_arity(crate::Arity::any().with_positionals())
            .unwrap();
        assert_eq!(positional.operands, vec![0]);
        let arguments = [Expanded, Literal("pattern"), Dynamic];
        let resolution = owner.commands().resolve_structured_invocation(
            crate::InvocationWords::structured(Literal("regexp"), &arguments),
            Some(owner.context().authoring_query()),
        );
        assert!(
            resolution
                .resolved()
                .unwrap()
                .authored_source_count_for_arity(crate::Arity::any().with_positionals())
                .is_none()
        );
    }

    #[test]
    fn source_expression_roles_keep_selected_tail_and_unknown_cardinality() {
        // naming.diagnostic.registry-source-ownership
        // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
        use crate::InvocationWord::{Dynamic, Expanded, Literal};
        let context = crate::model::ingress::static_context_for("tcl8.6");
        for (head, arguments, expected, concatenates) in [
            ("if", vec![Dynamic, Literal("{}")], vec![0], false),
            ("expr", vec![Dynamic, Literal("+"), Dynamic], vec![0], true),
            ("puts", vec![Dynamic], vec![], false),
        ] {
            let resolution = context.commands().resolve_structured_invocation(
                crate::InvocationWords::structured(Literal(head), &arguments),
                Some(context.context().authoring_query()),
            );
            let selected = resolution.resolved().unwrap();
            let expressions = selected
                .authored_source_expression_arguments()
                .unwrap_or_else(|| panic!("source role projection declined {head}"));
            assert_eq!(expressions.arguments, expected, "{head}");
            assert_eq!(expressions.concatenates, concatenates, "{head}");
        }
        let arguments = [Expanded];
        let resolution = context.commands().resolve_structured_invocation(
            crate::InvocationWords::structured(Literal("expr"), &arguments),
            Some(context.context().authoring_query()),
        );
        assert!(
            resolution
                .resolved()
                .unwrap()
                .authored_source_expression_arguments()
                .is_none()
        );
    }

    #[test]
    fn selected_argument_hints_retain_member_options_and_transparency() {
        use crate::InvocationWord::{Dynamic, Literal};
        let context = crate::model::ingress::static_context_for("tcl9.0");
        let registry = context.commands().clone();
        let dialect = crate::InvocationDialect::of_profile(registry.profile().unwrap());
        let arguments = [Literal("map"), Literal("-nocase"), Literal("a b"), Dynamic];
        let facts = registry
            .resolve_structured_invocation(
                crate::InvocationWords::structured(Literal("string"), &arguments)
                    .with_dialect(dialect),
                None,
            )
            .resolved()
            .unwrap()
            .facts();
        assert!(facts.argument_type_hint(0).is_none());
        assert!(facts.argument_type_hint(1).is_none());
        let mapping = facts.argument_type_hint(2).unwrap();
        assert_eq!(mapping.expected, Some(crate::TclType::List));
        assert!(mapping.shimmers);
        assert_eq!(mapping.transparent_from, &[crate::TclType::Dict]);
        assert_eq!(
            facts.argument_type_hint(3).unwrap().expected,
            Some(crate::TclType::String)
        );
        assert!(facts.argument_type_hint(4).is_none());
        assert_eq!(
            facts.operand_representation_coercions,
            crate::representation::RepresentationCoercionSelection::Operands {
                arguments: vec![2, 3],
            }
        );
        let length_arguments = [Literal("length"), Dynamic];
        let length = registry
            .resolve_structured_invocation(
                crate::InvocationWords::structured(Literal("string"), &length_arguments)
                    .with_dialect(dialect),
                None,
            )
            .resolved()
            .unwrap()
            .facts();
        let retained = length.clone();
        drop(length);
        drop(registry);
        assert_eq!(
            retained.argument_type_hint(1).unwrap().transparent_from,
            &[crate::TclType::ByteArray]
        );
    }

    #[test]
    fn selected_argument_hints_decline_unresolved_argv_and_rejected_arity() {
        use crate::InvocationWord::{Dynamic, Expanded, Literal};
        let context = crate::model::ingress::static_context_for("tcl9.0");
        let registry = context.commands();
        let dialect = crate::InvocationDialect::of_profile(registry.profile().unwrap());
        let cases: &[&[crate::InvocationWord<'_>]] = &[
            &[Dynamic, Literal("x")],
            &[Literal("map"), Dynamic, Literal("x")],
            &[Literal("length"), Expanded],
            &[Literal("length")],
            &[Literal("length"), Literal("x"), Literal("extra")],
        ];
        for arguments in cases {
            let facts = registry
                .resolve_structured_invocation(
                    crate::InvocationWords::structured(Literal("string"), arguments)
                        .with_dialect(dialect),
                    None,
                )
                .resolved()
                .unwrap()
                .facts();
            for index in 0..4 {
                assert!(
                    facts.argument_type_hint(index).is_none(),
                    "{arguments:?}: slot {index}"
                );
            }
        }
    }

    #[test]
    fn positional_arity_uses_proved_option_boundary_without_inventing_dynamic_values() {
        use crate::InvocationWord::{Dynamic, DynamicNonOption, Expanded, Literal};
        let registry = CommandRegistry::build_default();
        let cases: &[(&[crate::InvocationWord<'_>], Option<u16>)] = &[
            (
                &[Literal("create"), Literal("-safe"), Literal("jail")],
                Some(1),
            ),
            (
                &[Literal("create"), Literal("-safe"), Literal("-safe")],
                Some(0),
            ),
            (
                &[Literal("create"), Literal("--"), Literal("-safe")],
                Some(1),
            ),
            (&[Literal("create"), Literal("--"), Dynamic], Some(1)),
            (
                &[Literal("create"), Literal("-safe"), DynamicNonOption],
                Some(1),
            ),
            (&[Literal("create"), Dynamic], None),
            (&[Literal("create"), Literal("-safe"), Dynamic], None),
            (&[Literal("create"), Literal("-unknown")], None),
            (&[Literal("create"), Expanded], None),
            (
                &[Literal("create"), Literal("jail"), Literal("extra")],
                Some(2),
            ),
        ];
        for &(arguments, expected) in cases {
            let invocation = registry
                .resolve_structured_invocation(
                    crate::InvocationWords::structured(Literal("interp"), arguments),
                    None,
                )
                .resolved()
                .expect("literal interp selector resolves");
            assert_eq!(
                invocation.argument_count_for_arity(),
                expected,
                "{arguments:?}"
            );
            assert_eq!(
                invocation.facts().arity_accepts_frozen_arguments(),
                expected.map(|count| count <= 1),
                "{arguments:?}"
            );
        }
        let arguments = [Literal("-nonewline"), Dynamic];
        let invocation = registry
            .resolve_structured_invocation(
                crate::InvocationWords::structured(Literal("puts"), &arguments),
                None,
            )
            .resolved()
            .expect("literal puts resolves");
        assert_eq!(
            invocation.argument_count_for_arity(),
            Some(2),
            "raw argument signatures retain their authored count axis"
        );
    }

    #[test]
    fn structured_option_boundary_shares_alias_abbreviation_and_value_span_rules() {
        // Implementation contract: naming.studio.source-word-role-projection
        // docs/design/analysis/name-resolution-proofs/studio-source-word-role-projection.md

        use crate::InvocationWord::{Dynamic, Literal};
        let options = [
            OptionSpec {
                name: "-offset",
                aliases: &["-begin"],
                min_abbrev: Some(4),
                value: crate::hover::OptionValue::fixed(2, ArgRole::Value, "value"),
                ..OptionSpec::DEFAULT
            },
            OptionSpec {
                name: "--",
                ..OptionSpec::DEFAULT
            },
        ];
        let available = options.iter().collect::<Vec<_>>();
        let selected = InvocationOptions {
            availability: InvocationAvailability::default(),
            parent_surface: None,
            form_surface: None,
            prefix_matching: crate::abbrev::PrefixMatching::Enabled,
            positional_prefix_words: 0,
            reserved_trailing_words: 0,
            case_list: None,
            base: &options,
            form: &[],
        };
        for spelling in ["-offset", "-begin", "-off", "-beg"] {
            let arguments = [
                Literal(spelling),
                Literal("1"),
                Literal("2"),
                Literal("--"),
                Dynamic,
            ];
            assert_eq!(
                crate::spec::leading_option_word_count_for_arguments(
                    &available,
                    InvocationArguments::structured(&arguments),
                    crate::abbrev::PrefixMatching::Enabled,
                    0,
                ),
                Some(4),
                "{spelling}"
            );
            let occurrences = selected
                .prefix_occurrences(InvocationArguments::structured(&arguments))
                .unwrap();
            assert_eq!(occurrences.len(), 2);
            assert_eq!(occurrences[0].argument_index, 0);
            assert_eq!(occurrences[0].option.unwrap().name, "-offset");
            assert_eq!(occurrences[0].values, 1..3);
            assert_eq!(occurrences[1].argument_index, 3);
            assert!(occurrences[1].option.is_none());
            assert!(
                crate::spec::resolve_option_prefix_with(
                    &options,
                    spelling,
                    crate::abbrev::PrefixMatching::Enabled,
                )
                .is_some()
            );
        }
        assert!(
            crate::spec::resolve_option_prefix_with(
                &options,
                "-of",
                crate::abbrev::PrefixMatching::Enabled,
            )
            .is_none()
        );
        let unknown_value = [
            Literal("-offset"),
            Dynamic,
            Literal("2"),
            Literal("position"),
        ];
        assert_eq!(
            selected
                .prefix_occurrences(InvocationArguments::structured(&unknown_value))
                .unwrap()[0]
                .values,
            1..3
        );
        assert!(
            selected
                .prefix_occurrences(InvocationArguments::structured(&[
                    Dynamic,
                    Literal("position")
                ]))
                .is_none()
        );
        assert_eq!(
            crate::spec::leading_option_word_count_for_arguments(
                &available,
                InvocationArguments::structured(&unknown_value),
                crate::abbrev::PrefixMatching::Enabled,
                0,
            ),
            Some(3),
            "fixed value arity retains positions independently of dynamic bytes"
        );
        let terminator = [Literal("-offset"), Literal("--"), Dynamic];
        assert_eq!(
            crate::spec::leading_option_word_count_for_arguments(
                &available,
                InvocationArguments::structured(&terminator),
                crate::abbrev::PrefixMatching::Enabled,
                0,
            ),
            Some(3),
            "a dash value is consumed before subsequent option scanning"
        );
    }

    const COMMAND_CODES: &[CompletionCode] = &[CompletionCode::Error];
    const SUBCOMMAND_CODES: &[CompletionCode] = &[CompletionCode::Break];
    const FORM_CODES: &[CompletionCode] = &[CompletionCode::Other(71)];

    const COMMAND_WORLD_ACCESSES: &[StaticEffectAccess] = &[StaticEffectAccess::new(
        WorldStateDomain::PackageState,
        EffectAccessMode::Read,
        StaticInterpreterScope::Current,
        StaticNamespaceScope::Current,
        StaticSubjectScope::Wildcard,
    )];
    const SUBCOMMAND_WORLD_ACCESSES: &[StaticEffectAccess] = &[StaticEffectAccess::new(
        WorldStateDomain::OoDispatch,
        EffectAccessMode::Read,
        StaticInterpreterScope::Current,
        StaticNamespaceScope::Current,
        StaticSubjectScope::Wildcard,
    )];

    const COMMAND_WORLD: WorldEffectDescriptor = WorldEffectDescriptor {
        composition: WorldEffectComposition::Extend,
        static_footprint: StaticEffectFootprint {
            accesses: COMMAND_WORLD_ACCESSES,
            callback: crate::CallbackEffect::NONE,
        },
        resolver: None,
        dynamic_fallback: crate::WorldEffectDynamicFallback::ConservativeUnknownInvocation,
    };
    const SUBCOMMAND_WORLD: WorldEffectDescriptor = WorldEffectDescriptor {
        composition: WorldEffectComposition::Extend,
        static_footprint: StaticEffectFootprint {
            accesses: SUBCOMMAND_WORLD_ACCESSES,
            callback: crate::CallbackEffect::NONE,
        },
        resolver: None,
        dynamic_fallback: crate::WorldEffectDynamicFallback::ConservativeUnknownInvocation,
    };

    fn form_effect_resolver(args: InvocationArguments<'_>) -> EffectFootprint {
        let mut footprint = EffectFootprint::default();
        if let Some(subject) = args.literal_at(1) {
            footprint.add_access(EffectAccess::new(
                WorldStateDomain::VariableTraces,
                EffectAccessMode::Write,
                crate::InterpreterScope::Current,
                crate::NamespaceScope::Current,
                SubjectScope::named(subject),
            ));
        }
        footprint
    }

    const FORM_WORLD: WorldEffectDescriptor = WorldEffectDescriptor {
        composition: WorldEffectComposition::Extend,
        static_footprint: StaticEffectFootprint::EMPTY,
        resolver: Some(form_effect_resolver),
        dynamic_fallback: crate::WorldEffectDynamicFallback::ConservativeUnknownInvocation,
    };
    const REFINING_FORM_WORLD: WorldEffectDescriptor = WorldEffectDescriptor {
        composition: WorldEffectComposition::Replace,
        static_footprint: StaticEffectFootprint::EMPTY,
        resolver: Some(form_effect_resolver),
        dynamic_fallback: crate::WorldEffectDynamicFallback::ConservativeUnknownInvocation,
    };

    const WORLD_FORMS: &[CommandForm] = &[CommandForm {
        name: "argument-targeted",
        arity: Arity::exact(1),
        world_effects: Some(FORM_WORLD),
        ..CommandForm::DEFAULT
    }];
    const WORLD_SUBCOMMANDS: &[SubCommand] = &[
        SubCommand {
            name: "sub",
            arity: Arity::new(0, 1),
            subcommand_forms: WORLD_FORMS,
            world_effects: Some(SUBCOMMAND_WORLD),
            ..SubCommand::DEFAULT
        },
        SubCommand {
            name: "refine",
            arity: Arity::exact(1),
            subcommand_forms: &[CommandForm {
                name: "precise-target",
                arity: Arity::exact(1),
                world_effects: Some(REFINING_FORM_WORLD),
                ..CommandForm::DEFAULT
            }],
            ..SubCommand::DEFAULT
        },
    ];

    const RUN_FORMS: &[CommandForm] = &[CommandForm {
        name: "custom-code-form",
        arity: Arity::exact(0),
        completion: Some(CompletionDescriptor::exact(FORM_CODES)),
        ..CommandForm::DEFAULT
    }];

    const SUBCOMMANDS: &[SubCommand] = &[
        SubCommand {
            name: "run",
            arity: Arity::exact(0),
            completion: Some(CompletionDescriptor::exact(SUBCOMMAND_CODES)),
            subcommand_forms: RUN_FORMS,
            ..SubCommand::DEFAULT
        },
        SubCommand {
            name: "plain",
            arity: Arity::exact(0),
            completion: Some(CompletionDescriptor::exact(SUBCOMMAND_CODES)),
            ..SubCommand::DEFAULT
        },
    ];

    const SAFE_ON_UNINIT_SUBCOMMANDS: &[SubCommand] = &[
        SubCommand {
            name: "narrow",
            arity: Arity::exact(0),
            safe_on_uninit: Some(SpecSurface::TCL85_PLUS),
            ..SubCommand::DEFAULT
        },
        SubCommand {
            name: "inherit",
            arity: Arity::exact(0),
            ..SubCommand::DEFAULT
        },
    ];

    const OBJECT_DEPENDENCY: DispatchDependencies =
        DispatchDependencies::one(DispatchDependencyDomain::ObjectDispatch);
    const UNKNOWN_DEPENDENCY: DispatchDependencies =
        DispatchDependencies::one(DispatchDependencyDomain::UnknownHandling);
    const DISPATCH_FORMS: &[CommandForm] = &[CommandForm {
        name: "guarded-form",
        arity: Arity::exact(0),
        dispatch_dependencies: Some(DispatchDependencyDescriptor::replace(OBJECT_DEPENDENCY)),
        ..CommandForm::DEFAULT
    }];
    const DISPATCH_SUBCOMMANDS: &[SubCommand] = &[SubCommand {
        name: "run",
        arity: Arity::exact(0),
        subcommand_forms: DISPATCH_FORMS,
        dispatch_dependencies: Some(DispatchDependencyDescriptor::extend(UNKNOWN_DEPENDENCY)),
        ..SubCommand::DEFAULT
    }];

    fn registry_with_completion_fixture() -> CommandRegistry {
        let mut registry = CommandRegistry::build_default();
        registry.insert(CommandSpec {
            name: "completion-fixture",
            arity: Arity::any(),
            subcommands: SUBCOMMANDS,
            completion: Some(CompletionDescriptor::exact(COMMAND_CODES)),
            ..CommandSpec::DEFAULT
        });
        registry
    }

    #[test]
    fn retained_result_types_respect_option_forms_and_unknown_slots() {
        use crate::InvocationWord::{Dynamic, Literal};
        let registry = crate::model::ingress::static_context_for("tcl8.6").commands();
        for (command, arguments, expected) in [
            (
                "regexp",
                vec![Literal("-inline"), Literal("a"), Dynamic],
                None,
            ),
            ("regexp", vec![Literal("a"), Dynamic], Some(TclType::Int)),
            ("regexp", vec![Dynamic, Literal("a"), Dynamic], None),
            (
                "lsearch",
                vec![Literal("-all"), Dynamic, Literal("a")],
                None,
            ),
            ("lsearch", vec![Dynamic, Literal("a")], Some(TclType::Int)),
            ("lsearch", vec![Literal("--"), Dynamic, Literal("a")], None),
            (
                "regsub",
                vec![Literal("a"), Dynamic, Literal("b")],
                Some(TclType::String),
            ),
        ] {
            let invocation = registry.resolve_structured_invocation(
                crate::InvocationWords::structured(Literal(command), &arguments),
                registry.own_surface_query(),
            );
            assert_eq!(
                invocation.resolved().unwrap().facts().return_type,
                expected,
                "{command} {arguments:?}"
            );
        }
    }

    #[test]
    fn literal_leaf_normal_world_is_separate_from_error_and_dynamic_operands() {
        let registry = crate::model::ingress::static_context_for("tcl8.6").commands();
        let dialect = crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
        let arguments = [
            crate::InvocationWord::Literal("format"),
            crate::InvocationWord::Literal("a*"),
            crate::InvocationWord::Literal("hello"),
        ];
        let resolve = |arguments| {
            registry
                .resolve_structured_invocation(
                    crate::InvocationWords::structured(
                        crate::InvocationWord::Literal("binary"),
                        arguments,
                    )
                    .with_dialect(dialect),
                    registry.own_surface_query(),
                )
                .resolved()
                .unwrap()
        };
        let invocation = resolve(&arguments);
        assert!(invocation.facts().effects.requires_world_barrier());
        let normal = invocation.facts_after_success();
        assert!(!normal.effects.requires_world_barrier());
        assert!(
            normal
                .effects
                .accesses()
                .iter()
                .all(|access| access.domain == WorldStateDomain::InterpreterResult)
        );
        assert!(
            normal
                .state_transitions
                .declared()
                .unwrap()
                .facts()
                .is_empty()
        );
        let dynamic = [
            crate::InvocationWord::Literal("format"),
            crate::InvocationWord::Literal("a*"),
            crate::InvocationWord::Dynamic,
        ];
        assert!(
            resolve(&dynamic)
                .facts_after_success()
                .effects
                .requires_world_barrier()
        );
    }

    #[test]
    fn selected_getter_normal_contract_does_not_leak_to_mutator_or_channel_forms() {
        let registry = crate::model::ingress::static_context_for("f5-irules").commands();
        let dialect = crate::InvocationDialect::of_profile(tcl_dialect::DialectProfile::irules());
        for values in [&[][..], &["100"][..]] {
            let invocation = registry
                .resolve_structured_invocation(
                    crate::InvocationWords::literals("TCP::payload", values).with_dialect(dialect),
                    registry.own_surface_query(),
                )
                .resolved()
                .unwrap();
            assert_eq!(
                invocation.facts().successful_handler,
                Some(crate::native_compilation::SuccessfulHandlerSpec::Leaf)
            );
            assert!(
                !invocation
                    .facts_after_success()
                    .effects
                    .requires_world_barrier()
            );
            assert!(invocation.facts().native_compilation.is_none());
        }
        let replace = registry
            .resolve_invocation(
                "TCP::payload",
                &["replace", "0", "1", "data"],
                registry.own_surface_query(),
            )
            .unwrap();
        assert!(replace.facts().successful_handler.is_none());
        let registry = crate::model::ingress::static_context_for("tcl8.6").commands();
        let current = registry
            .resolve_invocation("pid", &[], registry.own_surface_query())
            .unwrap();
        assert_eq!(
            current.facts().successful_handler,
            Some(crate::native_compilation::SuccessfulHandlerSpec::Leaf)
        );
        let channel = registry
            .resolve_invocation("pid", &["channel"], registry.own_surface_query())
            .unwrap();
        assert!(channel.facts().successful_handler.is_none());
    }

    #[test]
    fn dictionary_constructor_coerces_keys_and_retains_unknown_value_objects() {
        let registry = crate::model::ingress::static_context_for("tcl8.6").commands();
        let dialect = crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
        for (key, value, closed) in [
            (
                crate::InvocationWord::Literal("key"),
                crate::InvocationWord::Dynamic,
                true,
            ),
            (
                crate::InvocationWord::Dynamic,
                crate::InvocationWord::Literal("value"),
                false,
            ),
        ] {
            let operands = [crate::InvocationWord::Literal("create"), key, value];
            let invocation = registry
                .resolve_structured_invocation(
                    crate::InvocationWords::structured(
                        crate::InvocationWord::Literal("dict"),
                        &operands,
                    )
                    .with_dialect(dialect),
                    registry.own_surface_query(),
                )
                .resolved()
                .unwrap();
            let effects = invocation.facts_after_success().effects;
            assert_eq!(!effects.requires_world_barrier(), closed);
            if closed {
                assert!(
                    effects
                        .accesses()
                        .iter()
                        .all(|access| access.domain == WorldStateDomain::InterpreterResult)
                );
            }
        }
    }

    #[test]
    fn repeating_target_roles_do_not_require_a_literal_container_value() {
        let registry = crate::model::ingress::static_context_for("tcl8.6").commands();
        let words = [
            crate::InvocationWord::Dynamic,
            crate::InvocationWord::Literal("x"),
            crate::InvocationWord::Literal("y"),
        ];
        let invocation = registry.resolve_structured_invocation(
            crate::InvocationWords::structured(crate::InvocationWord::Literal("lassign"), &words),
            registry.own_surface_query(),
        );
        let facts = invocation.resolved().unwrap().facts();
        assert!(facts.arg_roles_complete);
        assert_eq!(
            facts.arg_roles,
            vec![(1, ArgRole::VarWrite), (2, ArgRole::VarWrite)]
        );
    }

    #[test]
    fn completion_descriptor_precedence_is_form_then_subcommand_then_command() {
        let registry = registry_with_completion_fixture();

        let form = registry
            .resolve_invocation("completion-fixture", &["run"], None)
            .expect("fixture command resolves");
        assert_eq!(
            form.semantics.completion.codes,
            CompletionCodeDomain::Exact(FORM_CODES),
            "the matched form owns the most-specific descriptor"
        );

        let subcommand = registry
            .resolve_invocation("completion-fixture", &["plain"], None)
            .expect("fixture command resolves");
        assert_eq!(
            subcommand.semantics.completion.codes,
            CompletionCodeDomain::Exact(SUBCOMMAND_CODES),
            "a resolved subcommand overrides its parent command"
        );

        let command = registry
            .resolve_invocation("completion-fixture", &[], None)
            .expect("fixture command resolves without a subcommand");
        assert_eq!(
            command.semantics.completion.codes,
            CompletionCodeDomain::Exact(COMMAND_CODES),
            "the command descriptor applies when no narrower form resolved"
        );
    }

    #[test]
    fn registry_exposes_standard_code_descriptors_and_conservative_invoke_fallback() {
        let mut registry = CommandRegistry::build_default();

        let error = registry
            .resolve_invocation("error", &["boom"], None)
            .expect("error is a core command");
        assert_eq!(
            error.semantics.completion.codes,
            CompletionCodeDomain::Exact(&[CompletionCode::Error]),
            "the command registry, not a compiler spelling match, owns error's code"
        );

        registry.insert(CommandSpec {
            name: "completion-generic-fallback",
            ..CommandSpec::DEFAULT
        });
        let generic = registry
            .resolve_invocation("completion-generic-fallback", &[], None)
            .expect("unstamped fixture command resolves");
        assert_eq!(
            generic.semantics.completion,
            CompletionDescriptor::CONSERVATIVE,
            "an unstamped command must retain generic Invoke semantics"
        );
        assert_eq!(
            generic.semantics.operation,
            SemanticOperationId::Invoke,
            "the conservative descriptor does not enable a specialisation"
        );
        let generic_effects = generic.effects();
        assert!(generic_effects.callback().kinds.is_unknown());
        assert!(
            generic_effects.requires_world_barrier(),
            "unstamped generic Invoke must not be interpreted as pure"
        );
        let generic_transitions = generic.facts().state_transitions;

        registry.insert(CommandSpec {
            name: "explicit-closed-effect-fixture",
            world_effects: Some(WorldEffectDescriptor::EMPTY),
            ..CommandSpec::DEFAULT
        });
        let explicit_empty = registry
            .resolve_invocation("explicit-closed-effect-fixture", &[], None)
            .expect("explicitly effect-free fixture resolves")
            .effects();
        assert!(explicit_empty.accesses().is_empty());
        assert!(
            !explicit_empty.requires_world_barrier(),
            "the explicit EMPTY descriptor, not missing metadata, proves effect-freedom"
        );

        assert!(matches!(
            generic_transitions,
            StateTransitionKnowledge::UnknownInvocation
        ));

        registry.insert(CommandSpec {
            name: "explicit-closed-transition-fixture",
            state_transitions: Some(StateTransitionDescriptor::EMPTY),
            ..CommandSpec::DEFAULT
        });
        let explicit_transitions = registry
            .resolve_invocation("explicit-closed-transition-fixture", &[], None)
            .expect("explicitly transition-free fixture resolves")
            .facts()
            .state_transitions;
        assert!(matches!(
            explicit_transitions,
            StateTransitionKnowledge::Declared(ref transitions) if transitions.facts().is_empty()
        ));
    }

    #[test]
    fn scalar_math_shape_requires_the_selected_native_protocol() {
        let registry = CommandRegistry::build_default();
        let values = [crate::InvocationWord::Dynamic];
        for version in [
            tcl_dialect::TclVersion::V8_5,
            tcl_dialect::TclVersion::V8_6,
            tcl_dialect::TclVersion::V9_0,
            tcl_dialect::TclVersion::V9_1,
        ] {
            let dialect = crate::InvocationDialect::for_version(version);
            let words = crate::InvocationWords::structured(
                crate::InvocationWord::Literal("::tcl::mathfunc::sqrt"),
                &values,
            )
            .with_dialect(dialect);
            let resolution =
                registry.resolve_structured_invocation(words, dialect.authoring_query());
            let mut facts = resolution.resolved().unwrap().facts();
            let protocol = facts
                .normal_scalar_math_protocol(words.arguments())
                .unwrap();
            assert_eq!(protocol.operand_at(), 0);
            assert_eq!(
                protocol.numeric_operand_policy(),
                crate::mathfunc::NativeMathNumericOperandPolicy::PreserveCategory
            );
            assert_eq!(
                facts.normal_numeric_result_production(words.arguments()),
                Some(crate::native_result::NativeNumericResultProduction::Double)
            );
            assert_eq!(
                facts.completion.codes,
                crate::completion::CompletionCodeDomain::Exact(&[
                    CompletionCode::Ok,
                    CompletionCode::Error
                ])
            );
            let original_effects = facts.effects.clone();
            facts.refine_scalar_math_input_effects(words.arguments(), true);
            assert_eq!(facts.effects, original_effects);
            facts.refine_scalar_math_input_effects(words.arguments(), false);
            assert!(
                facts
                    .effects
                    .callback()
                    .kinds
                    .contains(crate::CallbackKinds::HOST)
            );
            assert!(!facts.traits.contains(Traits::PURE));
            assert_eq!(
                facts.normal_numeric_result_production(words.arguments()),
                Some(crate::native_result::NativeNumericResultProduction::Double)
            );
            facts.successful_handler = None;
            assert!(
                facts
                    .normal_scalar_math_protocol(words.arguments())
                    .is_none()
            );
            facts.successful_handler = Some(crate::native_compilation::SuccessfulHandlerSpec::Leaf);
            facts.native_result = None;
            assert!(
                facts
                    .normal_numeric_result_production(words.arguments())
                    .is_none()
            );
        }
        let expanded = [crate::InvocationWord::Expanded];
        let dialect = crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_1);
        let words = crate::InvocationWords::structured(
            crate::InvocationWord::Literal("::tcl::mathfunc::sqrt"),
            &expanded,
        )
        .with_dialect(dialect);
        let facts = registry
            .resolve_structured_invocation(words, dialect.authoring_query())
            .resolved()
            .unwrap()
            .facts();
        assert!(
            facts
                .normal_scalar_math_protocol(words.arguments())
                .is_none()
        );
        assert!(
            crate::native_result::NativeResultContract::ScalarMath(
                crate::mathfunc::NativeScalarMathOperation::Sqrt
            )
            .normal_numeric_result_production(crate::InvocationArguments::literals(&["4"]), 0)
            .is_none()
        );
    }

    #[test]
    fn native_list_length_numeric_result_requires_selected_contract_and_normal_identity() {
        let registry = CommandRegistry::build_default();
        let values = [crate::InvocationWord::Dynamic];
        let mut dialects = tcl_dialect::TclVersion::ALL
            .into_iter()
            .map(crate::InvocationDialect::for_version)
            .collect::<Vec<_>>();
        dialects.push(crate::InvocationDialect::of_point(
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
        ));
        for dialect in dialects {
            let words = crate::InvocationWords::structured(
                crate::InvocationWord::Literal("llength"),
                &values,
            )
            .with_dialect(dialect);
            let resolution =
                registry.resolve_structured_invocation(words, dialect.authoring_query());
            let mut facts = resolution.resolved().unwrap().facts();
            assert_eq!(
                facts.normal_numeric_result_production(words.arguments()),
                Some(
                    crate::native_result::NativeNumericResultProduction::Integer {
                        arithmetic: dialect.arithmetic().unwrap(),
                    }
                )
            );
            facts.operation = SemanticOperationId::Invoke;
            assert!(
                facts
                    .normal_numeric_result_production(words.arguments())
                    .is_none()
            );
            facts.operation = SemanticOperationId::Intrinsic(IntrinsicId::ListLength);
            facts.successful_handler = None;
            assert!(
                facts
                    .normal_numeric_result_production(words.arguments())
                    .is_none()
            );
        }
        let words = crate::InvocationWords::structured(
            crate::InvocationWord::Literal("llength"),
            &[crate::InvocationWord::Expanded],
        )
        .with_dialect(crate::InvocationDialect::for_version(
            tcl_dialect::TclVersion::V9_1,
        ));
        let resolution = registry.resolve_structured_invocation(words, None);
        assert!(
            resolution
                .resolved()
                .unwrap()
                .facts()
                .normal_numeric_result_production(words.arguments())
                .is_none()
        );
    }

    #[test]
    fn ordinary_list_length_completion_requires_the_selected_operation_and_handler() {
        use crate::representation::OrdinaryContainerRepresentation;
        for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let registry = crate::model::ingress::static_context_for(environment).commands();
            let profile = crate::model::ingress::resolve_environment(environment).unit_profile();
            let dialect = crate::InvocationDialect::of_profile(profile);
            let operands = [crate::InvocationWord::Dynamic];
            let words =
                InvocationWords::structured(crate::InvocationWord::Literal("llength"), &operands)
                    .with_dialect(dialect);
            let resolved = registry.resolve_structured_invocation(words, dialect.authoring_query());
            let mut facts = resolved.resolved().expect("native list length").facts();
            assert!(
                facts
                    .ordinary_list_length_completion(
                        words.arguments(),
                        0,
                        OrdinaryContainerRepresentation::Dictionary,
                    )
                    .is_some(),
                "{environment}"
            );
            facts.operation = SemanticOperationId::Invoke;
            assert!(
                facts
                    .ordinary_list_length_completion(
                        words.arguments(),
                        0,
                        OrdinaryContainerRepresentation::Dictionary,
                    )
                    .is_none()
            );
            facts.operation = SemanticOperationId::Intrinsic(IntrinsicId::ListLength);
            facts.successful_handler = None;
            assert!(
                facts
                    .ordinary_list_length_completion(
                        words.arguments(),
                        0,
                        OrdinaryContainerRepresentation::Dictionary,
                    )
                    .is_none()
            );
        }
        let mut registry = CommandRegistry::build_default();
        registry.insert(CommandSpec {
            name: "coercion-only",
            arity: Arity::exact(1),
            representation_effect: Some(RepresentationEffect::CoerceOrdinaryList { operand: 0 }),
            successful_handler: Some(crate::native_compilation::SuccessfulHandlerSpec::Leaf),
            ..CommandSpec::DEFAULT
        });
        let dialect = crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
        let words = InvocationWords::literals("coercion-only", &["a b"]).with_dialect(dialect);
        let resolved = registry.resolve_structured_invocation(words, dialect.authoring_query());
        assert!(
            resolved
                .resolved()
                .expect("metadata fixture")
                .facts()
                .ordinary_list_length_completion(
                    words.arguments(),
                    0,
                    OrdinaryContainerRepresentation::List,
                )
                .is_none()
        );
    }

    #[test]
    fn list_length_abstract_method_obligation_retains_native_axes() {
        use crate::representation::ListLengthObjectProtocol as Protocol;
        for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let registry = crate::model::ingress::static_context_for(environment).commands();
            let profile = crate::model::ingress::resolve_environment(environment).unit_profile();
            let dialect = crate::InvocationDialect::of_profile(profile);
            let operands = [crate::InvocationWord::Dynamic];
            let words =
                InvocationWords::structured(crate::InvocationWord::Literal("llength"), &operands)
                    .with_dialect(dialect);
            let resolved = registry.resolve_structured_invocation(words, dialect.authoring_query());
            let mut facts = resolved.resolved().expect("native list length").facts();
            let expected = if dialect
                .tcl_version
                .is_some_and(|version| version >= tcl_dialect::TclVersion::V9_0)
            {
                Protocol::AbstractLength { argument: 0 }
            } else {
                Protocol::Ordinary { argument: 0 }
            };
            assert_eq!(
                facts.list_length_object_protocol(words.arguments()),
                Some(expected),
                "{environment}"
            );
            assert_eq!(
                facts
                    .list_length_object_protocol(crate::InvocationArguments::structured(&operands)),
                Some(Protocol::Unknown)
            );
            facts.operation = SemanticOperationId::Invoke;
            assert_eq!(facts.list_length_object_protocol(words.arguments()), None);
            facts.operation = SemanticOperationId::Intrinsic(IntrinsicId::ListLength);
            facts.successful_handler = None;
            assert_eq!(facts.list_length_object_protocol(words.arguments()), None);
        }
    }

    #[test]
    fn semantic_operations_keep_structural_lowering_and_precise_leaf_intrinsics_distinct() {
        let registry = CommandRegistry::build_default();
        let dialect = Some(SurfaceQuery::core(Family::Tcl, "9.0"));

        for (command, arguments, lowering) in [
            ("incr", &["counter"][..], LoweringHookId::Incr),
            ("catch", &["body"][..], LoweringHookId::Catch),
            ("return", &[][..], LoweringHookId::Return),
            ("global", &["name"][..], LoweringHookId::Global),
        ] {
            let invocation = registry
                .resolve_invocation(command, arguments, dialect)
                .expect("core structural command resolves");
            assert_eq!(
                invocation.semantics.operation,
                SemanticOperationId::StructuredLowering(lowering)
            );
        }

        let llength = registry
            .resolve_invocation("llength", &["value"], dialect)
            .expect("llength resolves");
        assert_eq!(
            llength.semantics.operation,
            SemanticOperationId::Intrinsic(IntrinsicId::ListLength)
        );
        let lindex = registry
            .resolve_invocation("lindex", &["value", "0"], dialect)
            .expect("lindex resolves");
        assert_eq!(
            lindex.semantics.operation,
            SemanticOperationId::Intrinsic(IntrinsicId::ListIndex)
        );

        let dict_get = registry
            .resolve_invocation("dict", &["get", "value", "key"], dialect)
            .expect("dict get resolves");
        assert_eq!(
            dict_get.semantics.operation,
            SemanticOperationId::Intrinsic(IntrinsicId::DictGet)
        );

        let namespace_eval = registry
            .resolve_invocation("namespace", &["eval", "ns", "body"], dialect)
            .expect("namespace eval resolves");
        assert_eq!(
            namespace_eval.semantics.operation,
            SemanticOperationId::StructuredLowering(LoweringHookId::NamespaceEval)
        );

        let broad_string_family = registry
            .resolve_invocation("string", &["first", "a", "abc"], dialect)
            .expect("string first resolves");
        assert_eq!(
            broad_string_family.semantics.operation,
            SemanticOperationId::Invoke,
            "an unstamped emitter family is not a target-neutral intrinsic"
        );
    }

    #[test]
    fn world_effect_descriptors_compose_by_default_and_can_explicitly_refine() {
        let mut registry = CommandRegistry::build_default();
        registry.insert(CommandSpec {
            name: "world-effect-fixture",
            arity: Arity::any(),
            subcommands: WORLD_SUBCOMMANDS,
            world_effects: Some(COMMAND_WORLD),
            ..CommandSpec::DEFAULT
        });

        let command = registry
            .resolve_invocation("world-effect-fixture", &[], None)
            .expect("fixture command resolves");
        let command_effects = command.effects();
        assert_eq!(command_effects.accesses().len(), 1);
        assert_eq!(
            command_effects.accesses()[0].domain,
            WorldStateDomain::PackageState,
            "the command descriptor applies without a narrower match"
        );

        let subcommand = registry
            .resolve_invocation("world-effect-fixture", &["sub"], None)
            .expect("fixture subcommand resolves");
        let subcommand_effects = subcommand.effects();
        assert!(
            subcommand_effects
                .accesses()
                .iter()
                .any(|access| access.domain == WorldStateDomain::PackageState)
        );
        assert!(
            subcommand_effects
                .accesses()
                .iter()
                .any(|access| access.domain == WorldStateDomain::OoDispatch)
        );

        let form = registry
            .resolve_invocation("world-effect-fixture", &["sub", "targetCell"], None)
            .expect("fixture form resolves");
        let form_effects = form.effects();
        assert!(
            form_effects
                .accesses()
                .iter()
                .any(|access| access.domain == WorldStateDomain::PackageState)
        );
        assert!(
            form_effects
                .accesses()
                .iter()
                .any(|access| access.domain == WorldStateDomain::OoDispatch)
        );
        assert!(form_effects.accesses().iter().any(|access| {
            access.domain == WorldStateDomain::VariableTraces
                && access.subject == SubjectScope::named("targetCell")
        }));

        let refined = registry
            .resolve_invocation("world-effect-fixture", &["refine", "targetCell"], None)
            .expect("refining fixture form resolves");
        let refined_effects = refined.effects();
        assert_eq!(refined_effects.accesses().len(), 1);
        assert_eq!(
            refined_effects.accesses()[0].domain,
            WorldStateDomain::VariableTraces,
            "only an explicit Replace descriptor may discard inherited effects"
        );
        assert_eq!(
            refined_effects.accesses()[0].subject,
            SubjectScope::named("targetCell"),
            "the replacement descriptor is allowed to refine a wildcard parent subject"
        );
    }

    #[test]
    fn dynamic_effect_arguments_stay_conservative_without_exposing_a_value() {
        let mut registry = CommandRegistry::build_default();
        registry.insert(CommandSpec {
            name: "world-effect-fixture",
            arity: Arity::any(),
            subcommands: WORLD_SUBCOMMANDS,
            world_effects: Some(COMMAND_WORLD),
            ..CommandSpec::DEFAULT
        });
        let arguments = [
            crate::InvocationWord::Literal("sub"),
            crate::InvocationWord::Dynamic,
        ];
        let invocation = registry
            .resolve_structured_invocation(
                crate::InvocationWords::structured(
                    crate::InvocationWord::Literal("world-effect-fixture"),
                    &arguments,
                ),
                None,
            )
            .resolved()
            .expect("literal fixture command resolves");

        assert_eq!(
            invocation.form.map(|form| form.name),
            Some("argument-targeted"),
            "a dynamic non-expanded word preserves the arity without becoming a value"
        );
        let effects = invocation.effects();
        assert!(effects.requires_world_barrier());
        assert!(effects.callback().kinds.is_unknown());
        assert!(
            !effects.accesses().iter().any(|access| {
                access.domain == WorldStateDomain::VariableTraces
                    && access.subject != SubjectScope::Wildcard
            }),
            "the resolver cannot manufacture a named target from a dynamic word"
        );
    }

    #[test]
    fn invocation_facts_are_owned_and_target_neutral() {
        let mut registry = CommandRegistry::build_default();
        registry.insert(CommandSpec {
            name: "world-effect-fixture",
            arity: Arity::any(),
            subcommands: WORLD_SUBCOMMANDS,
            world_effects: Some(COMMAND_WORLD),
            ..CommandSpec::DEFAULT
        });
        let invocation = registry
            .resolve_invocation("world-effect-fixture", &["sub", "targetCell"], None)
            .expect("literal fixture command resolves");
        let facts = invocation.facts();

        assert_eq!(facts.canonical_command, "world-effect-fixture");
        assert_eq!(facts.subcommand.canonical_name(), Some("sub"));
        assert!(matches!(
            &facts.subcommand,
            OwnedSubcommandResolution::Exact {
                spelling,
                canonical_name,
            } if spelling == "sub" && canonical_name == "sub"
        ));
        assert_eq!(facts.form.as_deref(), Some("argument-targeted"));
        assert_eq!(facts.operation, SemanticOperationId::Invoke);
        assert_eq!(facts.completion, CompletionDescriptor::CONSERVATIVE);
        assert_eq!(facts.result_stability, ResultStability::Unknown);
        assert_eq!(facts.arity, Arity::exact(1));
        assert_eq!(facts.argument_offset, 1);
        assert!(facts.arg_roles.is_empty());
        assert!(facts.arg_roles_complete);
        assert_eq!(facts.return_type, None);
        assert_eq!(facts.var_write_typing, VarWriteTyping::ReturnValue);
        assert_eq!(facts.return_elements, None);
        assert_eq!(facts.var_elements_effect, None);
        assert_eq!(facts.body_kind, BodyKind::Plain);
        assert_eq!(facts.frame_effect, None);
        assert!(facts.effects.accesses().iter().any(|access| {
            access.domain == WorldStateDomain::VariableTraces
                && access.subject == SubjectScope::named("targetCell")
        }));
    }

    #[test]
    fn invocation_facts_expose_closed_referential_result_proofs() {
        let registry = CommandRegistry::build_default();
        let cases: &[(&str, &[&str])] = &[
            ("format", &["%s", "x"]),
            ("join", &["a b"]),
            ("lindex", &["a b", "0"]),
            ("linsert", &["a b", "0", "x"]),
            ("llength", &["a b"]),
            ("lrange", &["a b", "0", "end"]),
            ("lremove", &["a b", "0"]),
            ("lreplace", &["a b", "0", "0", "x"]),
            ("lreverse", &["a b"]),
            ("split", &["ab"]),
        ];

        for (command, arguments) in cases {
            let facts = registry
                .resolve_invocation(
                    command,
                    arguments,
                    Some(SurfaceQuery::core(Family::Tcl, "9.1")),
                )
                .unwrap_or_else(|| panic!("{command} resolves"))
                .facts();

            assert_eq!(
                facts.result_stability,
                ResultStability::ReferentiallyTransparent,
                "{command}"
            );
            assert!(
                facts.traits.contains(Traits::PURE | Traits::CSE_CANDIDATE),
                "{command}"
            );
            assert!(facts.effects.accesses().is_empty(), "{command}");
            assert!(!facts.effects.requires_world_barrier(), "{command}");
            assert!(
                matches!(
                    facts.state_transitions,
                    StateTransitionKnowledge::Declared(ref transitions)
                        if transitions.facts().is_empty()
                ),
                "{command}"
            );
        }
    }

    #[test]
    fn selected_scope_alias_traits_include_subcommand_declarations() {
        let registry = CommandRegistry::build_default();
        for (command, arguments, expected) in [
            ("namespace", &["upvar", "::ns", "source", "local"][..], true),
            ("dict", &["with", "dictionary", "puts $local"][..], true),
            (
                "dict",
                &["update", "dictionary", "key", "local", "puts $local"][..],
                true,
            ),
            ("my", &["variable", "local"][..], true),
            ("namespace", &["origin", "::p"][..], false),
        ] {
            let facts = registry
                .resolve_invocation(
                    command,
                    arguments,
                    Some(SurfaceQuery::core(Family::Tcl, "9.1")),
                )
                .unwrap_or_else(|| panic!("{command} {arguments:?} resolves"))
                .facts();
            assert_eq!(
                facts.traits.contains(Traits::CREATES_SCOPE_ALIAS),
                expected,
                "{command} {arguments:?}"
            );
        }
    }

    #[test]
    fn clock_result_dependencies_are_selected_by_subcommand() {
        let registry = CommandRegistry::build_default();
        let seconds = registry
            .resolve_invocation(
                "clock",
                &["seconds"],
                Some(SurfaceQuery::core(Family::Tcl, "9.0")),
            )
            .expect("clock seconds resolves")
            .facts();
        assert_eq!(seconds.result_stability, ResultStability::Volatile);
        assert!(
            seconds.traits.contains(Traits::PURE),
            "the established subcommand pure fact must reach common semantics"
        );
        assert!(seconds.effects.requires_world_barrier());
        assert_eq!(
            seconds.state_transitions,
            StateTransitionKnowledge::UnknownInvocation,
            "a result declaration must not fabricate effect or transition closure"
        );

        let add = registry
            .resolve_invocation(
                "clock",
                &["add", "0", "1", "day"],
                Some(SurfaceQuery::core(Family::Tcl, "9.0")),
            )
            .expect("clock add resolves")
            .facts();
        assert!(matches!(
            add.result_stability,
            ResultStability::ReadsVersionedWorld(_)
        ));
        let dependencies = add.result_stability.versioned_world_dependencies();
        assert!(dependencies.contains(&WorldStateDomain::HostCapabilities));
        assert!(dependencies.contains(&WorldStateDomain::PackageState));
        assert!(dependencies.contains(&WorldStateDomain::VariableStore));
        assert!(add.effects.requires_world_barrier());
    }

    #[test]
    fn optional_roles_follow_proved_argv_presence() {
        let registry = CommandRegistry::build_default();
        let query = Some(SurfaceQuery::core(Family::Tcl, "8.6"));
        for (arguments, expected) in [
            (vec!["{}"], vec![(0, ArgRole::Body)]),
            (
                vec!["{}", "result"],
                vec![(0, ArgRole::Body), (1, ArgRole::VarWrite)],
            ),
            (
                vec!["{}", "result", "options"],
                vec![
                    (0, ArgRole::Body),
                    (1, ArgRole::VarWrite),
                    (2, ArgRole::VarWrite),
                ],
            ),
        ] {
            let invocation = registry
                .resolve_invocation("catch", &arguments, query)
                .unwrap();
            assert_eq!(invocation.argument_roles().0, expected);
            assert_eq!(invocation.facts().arg_roles, expected);
            assert_eq!(invocation.facts_after_success().arg_roles, expected);
        }
        let words = [crate::InvocationWord::Expanded];
        let invocation = registry
            .resolve_structured_invocation(
                crate::InvocationWords::structured(crate::InvocationWord::Literal("catch"), &words),
                query,
            )
            .resolved()
            .unwrap();
        assert!(
            invocation
                .argument_roles()
                .0
                .contains(&(2, ArgRole::VarWrite))
        );
    }

    #[test]
    fn successful_frame_facts_are_separate_from_possible_argument_error_facts() {
        let registry = CommandRegistry::build_default();
        let arguments = [
            crate::InvocationWord::Dynamic,
            crate::InvocationWord::Literal("linked"),
        ];
        let words =
            crate::InvocationWords::structured(crate::InvocationWord::Literal("upvar"), &arguments)
                .with_dialect(crate::InvocationDialect::for_version(
                    tcl_dialect::TclVersion::V8_4,
                ));
        let invocation = registry
            .resolve_structured_invocation(words, Some(SurfaceQuery::core(Family::Tcl, "8.4")))
            .resolved()
            .expect("selected native upvar");
        let ordinary = invocation.facts();
        assert!(!ordinary.arg_roles_complete);
        assert!(
            !ordinary
                .state_transitions
                .declared()
                .unwrap()
                .facts()
                .iter()
                .any(|fact| { matches!(fact.transition, StateTransition::VariableCellAlias(_)) })
        );
        let normal = invocation.facts_after_success();
        assert!(normal.arg_roles_complete);
        assert_eq!(normal.arg_roles, vec![(1, ArgRole::VarWrite)]);
        assert!(
            normal
                .state_transitions
                .declared()
                .unwrap()
                .facts()
                .iter()
                .any(|fact| {
                    matches!(&fact.transition, StateTransition::VariableCellAlias(alias)
                    if alias.local.literal() == Some("linked")
                        && matches!(&alias.target, VariableAliasTarget::CallerSelectedFrame {
                            frame: CallerFrameSelection::DefaultCaller, ..
                        }))
                })
        );
        // Ordinary facts still retain the partial-commit error transfer; the
        // normal projection cannot replace them in caught-error states.
        assert!(
            ordinary
                .state_transitions
                .declared()
                .unwrap()
                .facts()
                .iter()
                .all(|fact| {
                    fact.commit.abrupt_transfer() == AbruptTransitionTransfer::JoinWithTransition
                })
        );
    }

    #[test]
    fn transition_facts_keep_literal_subjects_precise_and_dynamic_operands_scoped() {
        let registry = CommandRegistry::build_default();
        let arguments = [
            crate::InvocationWord::Literal("::precise"),
            crate::InvocationWord::Dynamic,
            crate::InvocationWord::Dynamic,
        ];
        let invocation = registry
            .resolve_structured_invocation(
                crate::InvocationWords::structured(
                    crate::InvocationWord::Literal("proc"),
                    &arguments,
                ),
                Some(SurfaceQuery::core(Family::Tcl, "8.6")),
            )
            .resolved()
            .expect("literal proc resolves");
        let facts = invocation.facts();

        let [transition] = facts
            .state_transitions
            .declared()
            .expect("proc declares state transitions")
            .facts()
        else {
            panic!("proc must emit exactly one definition transition");
        };
        assert_eq!(transition.commit, StateTransitionCommit::OnOkOnly);
        assert_eq!(
            transition.commit.abrupt_transfer(),
            AbruptTransitionTransfer::Unchanged,
            "a failed proc must not be treated as an unconditional definition"
        );
        assert_eq!(
            transition.transition,
            StateTransition::CommandBinding(CommandBindingTransition::Define {
                name: TransitionSubject::LocatedLiteral {
                    value: "::precise".to_owned(),
                    argument_index: 0
                },
                kind: CommandBindingDefinitionKind::Procedure,
            })
        );
    }

    #[test]
    fn expanded_positional_transition_grammar_suppresses_precise_facts() {
        let registry = CommandRegistry::build_default();
        let arguments = [
            crate::InvocationWord::Literal("::maybe"),
            crate::InvocationWord::Expanded,
            crate::InvocationWord::Literal("body"),
        ];
        let invocation = registry
            .resolve_structured_invocation(
                crate::InvocationWords::structured(
                    crate::InvocationWord::Literal("proc"),
                    &arguments,
                ),
                Some(SurfaceQuery::core(Family::Tcl, "8.6")),
            )
            .resolved()
            .expect("literal proc resolves despite an expanded argument");
        let facts = invocation.facts();

        let transitions = facts
            .state_transitions
            .declared()
            .expect("proc declares state transitions");
        assert!(
            transitions.facts().iter().all(|fact| !matches!(
                fact.transition,
                StateTransition::CommandBinding(CommandBindingTransition::Define { .. })
            )),
            "an expanded argv segment can change proc's positional grammar"
        );
        assert!(transitions.facts().iter().any(|fact| {
            matches!(
                fact.transition,
                StateTransition::Widen(ref widening)
                    if widening.subject
                        == TransitionSubject::Unknown {
                            argument_index: 1,
                            word_kind: crate::InvocationWordKind::Expanded,
                        }
            )
        }));
    }

    #[test]
    fn namespace_eval_and_dynamic_delete_materialise_closed_namespace_facts() {
        let registry = CommandRegistry::build_default();
        let eval = registry
            .resolve_invocation(
                "namespace",
                &["eval", "::a", "error x"],
                Some(SurfaceQuery::core(Family::Tcl, "8.6")),
            )
            .expect("namespace eval resolves")
            .facts();
        let [ensure] = eval
            .state_transitions
            .declared()
            .expect("namespace eval declares transitions")
            .facts()
        else {
            panic!("namespace eval must emit exactly one ensure transition");
        };
        assert_eq!(
            ensure.commit,
            StateTransitionCommit::MayCommitBeforeAbruptCompletion
        );
        assert!(matches!(
            &ensure.transition,
            StateTransition::Namespace(NamespaceTransition::Ensure {
                namespace: NamespaceTransitionTarget::Named(TransitionSubject::LocatedLiteral { value: name, .. }),
            }) if name == "::a"
        ));
        assert!(
            eval.effects
                .callback()
                .kinds
                .contains(CallbackKinds::SCRIPT)
        );
        assert_eq!(
            eval.effects.callback().reentrancy,
            Reentrancy::CurrentInterpreter
        );

        let arguments = [
            crate::InvocationWord::Literal("delete"),
            crate::InvocationWord::Dynamic,
        ];
        let delete = registry
            .resolve_structured_invocation(
                crate::InvocationWords::structured(
                    crate::InvocationWord::Literal("namespace"),
                    &arguments,
                ),
                Some(SurfaceQuery::core(Family::Tcl, "8.6")),
            )
            .resolved()
            .expect("literal namespace delete resolves")
            .facts();
        let transitions = delete
            .state_transitions
            .declared()
            .expect("namespace delete declares transitions");
        assert!(transitions.facts().iter().any(|fact| {
            matches!(
                &fact.transition,
                StateTransition::Widen(widening)
                    if widening.domains.contains(&StateTransitionDomain::ObjectDispatch)
            )
        }));
    }

    #[test]
    fn interp_alias_query_and_delete_do_not_claim_alias_creation() {
        let registry = CommandRegistry::build_default();
        let query = registry
            .resolve_invocation(
                "interp",
                &["alias", "", "shortcut"],
                Some(SurfaceQuery::core(Family::Tcl, "8.6")),
            )
            .expect("interp alias query resolves")
            .facts();
        assert!(
            query
                .state_transitions
                .declared()
                .expect("interp alias declares transitions")
                .facts()
                .is_empty()
        );

        let delete = registry
            .resolve_invocation(
                "interp",
                &["alias", "", "shortcut", ""],
                Some(SurfaceQuery::core(Family::Tcl, "8.6")),
            )
            .expect("interp alias delete resolves")
            .facts();
        let [transition] = delete
            .state_transitions
            .declared()
            .expect("interp alias declares transitions")
            .facts()
        else {
            panic!("alias delete must emit one deletion transition");
        };
        assert!(matches!(
            &transition.transition,
            StateTransition::CommandBinding(CommandBindingTransition::Delete {
                interpreter: Some(TransitionSubject::LocatedLiteral { value: path, .. }),
                name: TransitionSubject::LocatedLiteral { value: name, .. },
            }) if path.is_empty() && name == "shortcut"
        ));
        assert_eq!(
            transition.commit,
            StateTransitionCommit::MayCommitBeforeAbruptCompletion
        );
    }

    #[test]
    fn frame_and_namespace_alias_transitions_follow_registry_layouts() {
        let registry = CommandRegistry::build_default();
        let upvar = registry
            .resolve_invocation(
                "upvar",
                &["1", "other", "local"],
                Some(SurfaceQuery::core(Family::Tcl, "8.6")),
            )
            .expect("upvar resolves")
            .facts();
        assert!(matches!(
            upvar
                .state_transitions
                .declared()
                .expect("upvar declares transitions")
                .facts(),
            [transition]
                if matches!(
                    &transition.transition,
                    StateTransition::VariableCellAlias(alias)
                        if alias.local == TransitionSubject::LocatedLiteral { value: "local".to_owned(), argument_index: 2 }
                            && alias.target == VariableAliasTarget::CallerSelectedFrame {
                                frame: CallerFrameSelection::Explicit(
                                    TransitionSubject::LocatedLiteral { value: "1".to_owned(), argument_index: 0 }
                                ),
                                variable: TransitionSubject::LocatedLiteral { value: "other".to_owned(), argument_index: 1 },
                            }
                )
        ));

        let namespace_upvar = registry
            .resolve_invocation(
                "namespace",
                &["upvar", "::scope", "other", "local"],
                Some(SurfaceQuery::core(Family::Tcl, "8.6")),
            )
            .expect("namespace upvar resolves")
            .facts();
        assert!(matches!(
            namespace_upvar
                .state_transitions
                .declared()
                .expect("namespace upvar declares transitions")
                .facts(),
            [transition]
                if matches!(
                    &transition.transition,
                    StateTransition::VariableCellAlias(alias)
                        if alias.local == TransitionSubject::LocatedLiteral { value: "local".to_owned(), argument_index: 3 }
                            && alias.target == VariableAliasTarget::Namespace {
                                namespace: TransitionSubject::LocatedLiteral { value: "::scope".to_owned(), argument_index: 1 },
                                variable: TransitionSubject::LocatedLiteral { value: "other".to_owned(), argument_index: 2 },
                            }
                )
        ));
    }

    fn resolver_backed_arg_roles(_: &[&str]) -> Vec<(u8, ArgRole)> {
        vec![(1, ArgRole::VarRead)]
    }

    #[test]
    fn invocation_facts_resolve_roles_for_literal_argv() {
        let mut registry = CommandRegistry::build_default();
        registry.insert(CommandSpec {
            name: "resolver-backed-roles-fixture",
            arity: Arity::any(),
            arg_roles: &[(0, ArgRole::VarWrite)],
            arg_role_resolver: Some(resolver_backed_arg_roles),
            ..CommandSpec::DEFAULT
        });
        let facts = registry
            .resolve_invocation("resolver-backed-roles-fixture", &["first", "second"], None)
            .expect("fixture command resolves")
            .facts();

        assert_eq!(facts.arg_roles, vec![(1, ArgRole::VarRead)]);
        assert!(facts.arg_roles_complete);
    }

    #[test]
    fn sole_argument_role_index_uses_resolved_array_shape_and_dialect() {
        let registry = CommandRegistry::build_default();
        let variable_roles = [ArgRole::VarRead, ArgRole::VarWrite];

        let exists = registry
            .resolve_invocation(
                "array",
                &["exists", "items"],
                Some(SurfaceQuery::core(Family::Tcl, "9.0")),
            )
            .expect("array exists resolves")
            .facts();
        assert_eq!(
            exists.sole_argument_index_for_roles(2, &variable_roles),
            Some(1)
        );

        let default = registry
            .resolve_invocation(
                "array",
                &["def", "get", "items"],
                Some(SurfaceQuery::core(Family::Tcl, "9.0")),
            )
            .expect("Tcl 9 array default prefix resolves")
            .facts();
        assert_eq!(default.subcommand.canonical_name(), Some("default"));
        assert_eq!(
            default.sole_argument_index_for_roles(3, &variable_roles),
            Some(2)
        );
        assert_eq!(
            default.sole_argument_index_for_roles(5, &variable_roles),
            None,
            "the member's outer arity is part of target resolution"
        );

        let legacy = registry
            .resolve_invocation(
                "array",
                &["default", "get", "items"],
                Some(SurfaceQuery::core(Family::Tcl, "8.6")),
            )
            .expect("the array command itself resolves")
            .facts();
        assert_eq!(legacy.subcommand.canonical_name(), None);
        assert_eq!(
            legacy.sole_argument_index_for_roles(3, &variable_roles),
            None,
            "a Tcl 9-only member has no target under Tcl 8.6"
        );

        let dict_with = registry
            .resolve_invocation(
                "dict",
                &["with", "items", ""],
                Some(SurfaceQuery::core(Family::Tcl, "9.0")),
            )
            .expect("dict with resolves")
            .facts();
        assert!(dict_with.arg_roles_complete);
        assert_eq!(
            dict_with.sole_argument_index_for_roles(3, &variable_roles),
            Some(1),
            "one multi-role argument is still the sole matching operand"
        );
    }

    #[test]
    fn invocation_facts_keep_dynamic_resolver_roles_incomplete() {
        let mut registry = CommandRegistry::build_default();
        registry.insert(CommandSpec {
            name: "resolver-backed-dynamic-roles-fixture",
            arity: Arity::any(),
            arg_roles: &[(0, ArgRole::VarWrite)],
            arg_role_resolver: Some(resolver_backed_arg_roles),
            arg_role_resolver_roles: &[ArgRole::VarRead],
            ..CommandSpec::DEFAULT
        });
        let arguments = [
            crate::InvocationWord::Literal("first"),
            crate::InvocationWord::Dynamic,
        ];
        let facts = registry
            .resolve_structured_invocation(
                crate::InvocationWords::structured(
                    crate::InvocationWord::Literal("resolver-backed-dynamic-roles-fixture"),
                    &arguments,
                ),
                None,
            )
            .resolved()
            .expect("fixture command resolves")
            .facts();

        assert_eq!(facts.arg_roles, vec![(0, ArgRole::VarWrite)]);
        assert!(!facts.arg_roles_complete);
        assert_eq!(facts.arg_role_resolver_roles, &[ArgRole::VarRead]);
    }

    #[test]
    fn invocation_facts_keep_conservative_dispatch_dependencies_by_default() {
        let facts = {
            let mut registry = CommandRegistry::build_default();
            registry.insert(CommandSpec {
                name: "unstamped-dispatch-fixture",
                arity: Arity::exact(0),
                ..CommandSpec::DEFAULT
            });
            registry
                .resolve_invocation("unstamped-dispatch-fixture", &[], None)
                .expect("fixture command resolves")
                .facts()
        };

        assert_eq!(
            facts.dispatch_dependencies,
            DispatchDependencies::CONSERVATIVE
        );
        assert!(
            facts
                .dispatch_dependencies
                .contains(DispatchDependencyDomain::UnknownHandling),
            "static registry resolution is not itself a live-dispatch proof"
        );
    }

    #[test]
    fn safe_on_uninit_resolves_from_the_matched_subcommand() {
        let mut registry = CommandRegistry::build_default();
        registry.insert(CommandSpec {
            name: "safe-on-uninit-fixture",
            arity: Arity::any(),
            safe_on_uninit: Some(SpecSurface::ALL_TCL),
            subcommands: SAFE_ON_UNINIT_SUBCOMMANDS,
            ..CommandSpec::DEFAULT
        });

        let narrow = registry
            .resolve_invocation("safe-on-uninit-fixture", &["narrow"], None)
            .expect("fixture subcommand resolves");
        assert_eq!(
            narrow.semantics.safe_on_uninit,
            Some(SpecSurface::TCL85_PLUS)
        );

        let inherited = registry
            .resolve_invocation("safe-on-uninit-fixture", &["inherit"], None)
            .expect("fixture subcommand resolves");
        assert_eq!(
            inherited.semantics.safe_on_uninit,
            Some(SpecSurface::ALL_TCL)
        );
    }

    #[test]
    fn invocation_facts_own_composed_command_subcommand_and_form_dependencies() {
        let facts = {
            let mut registry = CommandRegistry::build_default();
            registry.insert(CommandSpec {
                name: "dispatch-composition-fixture",
                arity: Arity::any(),
                subcommands: DISPATCH_SUBCOMMANDS,
                dispatch_dependencies: Some(DispatchDependencyDescriptor::replace(
                    OBJECT_DEPENDENCY,
                )),
                ..CommandSpec::DEFAULT
            });
            registry
                .resolve_invocation("dispatch-composition-fixture", &["run"], None)
                .expect("fixture command resolves")
                .facts()
        };

        // Command replace(object) is extended by the subcommand's unknown
        // requirement, then the form replaces that refinable portion with
        // object-only. The irreducible live-interpreter base remains.
        assert_eq!(
            facts.dispatch_dependencies,
            DispatchDependencies::BASE.union(OBJECT_DEPENDENCY)
        );
        assert!(
            !facts
                .dispatch_dependencies
                .contains(DispatchDependencyDomain::UnknownHandling)
        );
        assert_eq!(facts.form.as_deref(), Some("guarded-form"));
        assert_eq!(facts.subcommand.canonical_name(), Some("run"));
    }

    #[test]
    fn transition_coverage_replaces_only_the_duplicate_legacy_write() {
        let registry = CommandRegistry::build_default();
        let invocation = registry
            .resolve_invocation(
                "rename",
                &["old", "new"],
                Some(SurfaceQuery::core(Family::Tcl, "8.6")),
            )
            .expect("core registry command resolves");
        let footprint = invocation.effects();

        assert!(
            !footprint.accesses().iter().any(|access| {
                access.domain == WorldStateDomain::CommandBindings
                    && matches!(
                        access.mode,
                        EffectAccessMode::Write | EffectAccessMode::ReadWrite
                    )
            }),
            "the completion-edge transition, not an unconditional legacy write, owns bindings"
        );
        assert!(footprint.accesses().iter().any(|access| {
            access.domain == WorldStateDomain::CommandTraces
                && access.mode == EffectAccessMode::Read
        }));
        assert!(
            footprint.legacy().command_table_mutation,
            "the command-table mutation the transition states remains visible \
             to the legacy effect bridge"
        );
        assert_eq!(footprint.legacy().side_effects.len(), 1);

        let facts = invocation.facts();
        assert!(
            !facts.transition_effect_coverage.entries().is_empty(),
            "the owned hand-off retains the registry coverage contract"
        );
        assert!(
            facts
                .effects
                .callback()
                .kinds
                .contains(CallbackKinds::TRACE)
        );
    }

    #[test]
    fn transition_coverage_does_not_hide_an_uncovered_variable_value_write() {
        let registry = CommandRegistry::build_default();
        let variable = registry
            .resolve_invocation(
                "variable",
                &["name", "value"],
                Some(SurfaceQuery::core(Family::Tcl, "8.6")),
            )
            .expect("variable resolves")
            .facts();
        assert!(
            variable.transition_effect_coverage.entries().is_empty(),
            "the alias transition does not claim to model value initialisation"
        );
        assert!(variable.effects.accesses().iter().any(|access| {
            access.domain == WorldStateDomain::VariableStore
                && matches!(
                    access.mode,
                    EffectAccessMode::Write | EffectAccessMode::ReadWrite
                )
        }));
    }
    #[test]
    fn no_value_append_forms_do_not_donate_mutation_contracts() {
        for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let registry = crate::model::ingress::static_context_for(environment).commands();
            let profile = crate::model::ingress::resolve_environment(environment).unit_profile();
            let dialect = crate::InvocationDialect::of_profile(profile);
            for command in ["append", "lappend"] {
                let words = InvocationWords::literals(command, &["x"]).with_dialect(dialect);
                let resolved =
                    registry.resolve_structured_invocation(words, dialect.authoring_query());
                let facts = resolved.resolved().expect("selected native form").facts();
                assert_eq!(
                    facts.operation,
                    SemanticOperationId::Invoke,
                    "{environment}: {command}"
                );
                assert!(!facts.traits.contains(Traits::UNCONDITIONAL_VARIABLE_WRITE));
                assert_eq!(facts.return_type, None);
                assert_eq!(facts.byte_array_effect, crate::ByteArrayEffect::None);
                assert_eq!(facts.var_elements_effect, None);
                assert_eq!(
                    facts.native_result,
                    Some(crate::native_result::NativeResultContract::VariableValue {
                        variable_at: 0,
                        phase: crate::native_result::VariableResultPhase::AfterRead,
                    })
                );
                if command == "append" {
                    assert_eq!(facts.arg_roles, [(0, ArgRole::VarRead)]);
                    assert_eq!(
                        resolved
                            .resolved()
                            .expect("read form")
                            .semantics
                            .safe_on_uninit,
                        None
                    );
                    assert!(registry.arg_type_hint_words(words, 0).is_none());
                } else {
                    assert!(facts.traits.contains(Traits::CONDITIONAL_VARIABLE_WRITE));
                    assert_eq!(facts.successful_handler, Some(crate::native_compilation::SuccessfulHandlerSpec::InitialiseEmptyVariable));
                    assert_eq!(
                        registry.arg_type_hint_words(words, 0).is_some(),
                        environment != "jim"
                    );
                }
                let mutating = registry
                    .resolve_invocation(command, &["x", "value"], dialect.authoring_query())
                    .expect("mutating native form")
                    .facts();
                assert!(
                    mutating
                        .traits
                        .contains(Traits::UNCONDITIONAL_VARIABLE_WRITE)
                );
            }
        }
    }
    #[test]
    fn cardinality_roles_preserve_dynamic_foreach_operands_and_unknown_expansion() {
        use crate::InvocationWord::{Dynamic, Expanded, Literal};
        let registry = CommandRegistry::build_default();
        for (arguments, body, complete) in [
            (
                vec![Literal("x"), Dynamic, Literal("set y $x")],
                Some(2),
                true,
            ),
            (
                vec![Literal("x"), Dynamic, Literal("y"), Dynamic, Dynamic],
                Some(4),
                true,
            ),
            (
                vec![Literal("x"), Expanded, Literal("set y $x")],
                None,
                false,
            ),
        ] {
            let invocation = registry
                .resolve_structured_invocation(
                    crate::InvocationWords::structured(Literal("foreach"), &arguments),
                    Some(SurfaceQuery::core(Family::Tcl, "8.6")),
                )
                .resolved()
                .expect("selected foreach");
            assert_eq!(
                invocation.semantics.arg_role_resolver_input(),
                crate::spec::ArgRoleResolverInput::Cardinality
            );
            let facts = invocation.facts();
            assert_eq!(
                facts
                    .arg_roles
                    .iter()
                    .find_map(|(at, role)| (*role == ArgRole::Body).then_some(*at)),
                body
            );
            assert_eq!(facts.arg_roles_complete, complete);
        }
    }

    #[test]
    fn cardinality_scan_roles_preserve_unknown_values_and_expansion_uncertainty() {
        use crate::InvocationWord::{Dynamic, Expanded, Literal};
        for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let registry = crate::model::ingress::static_context_for(environment).commands();
            let profile = crate::model::ingress::resolve_environment(environment).unit_profile();
            let dialect = crate::InvocationDialect::of_profile(profile);
            for command in ["scan", "binary"] {
                if command == "binary" && environment == "jim" {
                    continue;
                }
                let prefix = usize::from(command == "binary");
                let mut operands = Vec::new();
                if prefix != 0 {
                    operands.push(Literal("scan"));
                }
                operands.extend([Dynamic, Dynamic, Literal("first"), Literal("second")]);
                let words =
                    InvocationWords::structured(Literal(command), &operands).with_dialect(dialect);
                let resolved =
                    registry.resolve_structured_invocation(words, dialect.authoring_query());
                let facts = resolved.resolved().expect("selected native scan").facts();
                assert!(facts.arg_roles_complete, "{environment}: {command}");
                assert_eq!(
                    facts.arg_roles,
                    vec![
                        (1, ArgRole::ScanFormat),
                        (2, ArgRole::VarWrite),
                        (3, ArgRole::VarWrite),
                    ],
                    "{environment}: {command}"
                );
                operands[prefix] = Expanded;
                let words =
                    InvocationWords::structured(Literal(command), &operands).with_dialect(dialect);
                let resolved =
                    registry.resolve_structured_invocation(words, dialect.authoring_query());
                let facts = resolved
                    .resolved()
                    .expect("selected scan with unknown cardinality")
                    .facts();
                assert!(!facts.arg_roles_complete);
                assert!(
                    !facts
                        .arg_roles
                        .iter()
                        .any(|(_, role)| *role == ArgRole::VarWrite)
                );
            }
        }
    }

    #[test]
    fn positional_count_preserves_native_reserved_tail_and_case_layout() {
        use crate::InvocationWord::{Dynamic, Literal};
        for version in tcl_dialect::TclVersion::ALL {
            let dialect = crate::InvocationDialect::for_version(version);
            let registry = crate::CommandRegistry::build_default();
            let cases: &[(&str, &[crate::InvocationWord<'_>], u16)] = &[
                ("lsort", &[Literal("-integer"), Dynamic], 1),
                ("lsort", &[Literal("-integer")], 1),
                ("lsearch", &[Literal("-exact"), Dynamic, Dynamic], 2),
                ("lsearch", &[Literal("-exact"), Dynamic], 2),
                (
                    "switch",
                    &[Literal("-exact"), Literal("--"), Dynamic, Literal("x {}")],
                    2,
                ),
            ];
            for &(head, arguments, expected) in cases {
                let words = crate::InvocationWords::structured(Literal(head), arguments)
                    .with_dialect(dialect);
                let invocation = registry
                    .resolve_structured_invocation(words, dialect.authoring_query())
                    .resolved()
                    .expect("native literal head");
                assert_eq!(
                    invocation.argument_count_for_arity(),
                    Some(expected),
                    "{version:?} {head}"
                );
                assert_eq!(
                    invocation.facts().arity_accepts_frozen_arguments(),
                    Some(true)
                );
            }
            let arguments = [Literal("-exact"), Literal("-exact {}")];
            let invocation = registry
                .resolve_structured_invocation(
                    crate::InvocationWords::structured(Literal("switch"), &arguments)
                        .with_dialect(dialect),
                    dialect.authoring_query(),
                )
                .resolved()
                .expect("native switch");
            assert_eq!(
                invocation.argument_count_for_arity(),
                if version == tcl_dialect::TclVersion::V8_4 {
                    None
                } else {
                    Some(2)
                }
            );
        }
    }
}
#[test]
// Implementation contract: naming.variable.registry-receiver-authoring-parity
// docs/design/analysis/name-resolution-proofs/registry-variable-receiver-authoring-parity.md
fn variable_receiver_fact_projection_preserves_form_precedence_and_member_offsets() {
    use VariableReceiverOperandForm::Combined;
    const FORMS: &[CommandForm] = &[
        CommandForm {
            name: "inherited",
            arity: Arity::exact(1),
            arg_roles: &[(0, ArgRole::VarWrite)],
            ..CommandForm::DEFAULT
        },
        CommandForm {
            name: "withdrawn",
            arity: Arity::exact(2),
            arg_roles: &[(0, ArgRole::VarWrite)],
            variable_receivers: Some(&[]),
            ..CommandForm::DEFAULT
        },
        CommandForm {
            name: "selected",
            arity: Arity::exact(3),
            arg_roles: &[(2, ArgRole::VarWrite)],
            variable_receivers: Some(&[(2, Combined)]),
            ..CommandForm::DEFAULT
        },
    ];
    const MEMBERS: &[SubCommand] = &[SubCommand {
        name: "nested",
        arity: Arity::exact(1),
        arg_roles: &[(0, ArgRole::VarRead)],
        variable_receivers: Some(&[(0, Combined)]),
        ..SubCommand::DEFAULT
    }];
    let mut registry = crate::CommandRegistry::build_default();
    registry.insert(CommandSpec {
        name: "receiver-forms",
        variable_receivers: Some(&[(0, Combined)]),
        command_forms: FORMS,
        ..CommandSpec::DEFAULT
    });
    registry.insert(CommandSpec {
        name: "receiver-member",
        subcommands: MEMBERS,
        ..CommandSpec::DEFAULT
    });
    let dialect = crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
    for (args, position, expected) in [
        (vec!["x"], 0, Some(Combined)),
        (vec!["x", "V"], 0, None),
        (vec!["a", "b", "x"], 2, Some(Combined)),
    ] {
        let selected = registry
            .resolve_invocation("receiver-forms", &args, dialect.authoring_query())
            .unwrap();
        let facts = selected.facts();
        assert_eq!(facts.variable_receiver_operand_form(position), expected);
        assert!(facts.successful_handler.is_none());
        assert!(facts.native_compilation.is_none());
    }
    let selected = registry
        .resolve_invocation(
            "receiver-member",
            &["nested", "x"],
            dialect.authoring_query(),
        )
        .unwrap();
    let facts = selected.facts();
    assert_eq!(facts.argument_offset, 1);
    assert_eq!(facts.variable_receiver_operand_form(1), Some(Combined));
    assert!(facts.variable_receiver_operand_form(0).is_none());
}

#[test]
// Implementation contract: naming.variable.registry-receiver-authoring-parity
// docs/design/analysis/name-resolution-proofs/registry-variable-receiver-authoring-parity.md
fn selected_variable_receiver_form_keeps_array_argv_combined_and_aliases_separate() {
    // Native proof: naming.array-source.combined-set-receiver
    // docs/design/analysis/name-resolution-proofs/array-source-combined-set-receiver.md
    // Native proof: naming.array-source.scalar-element-storage
    // docs/design/analysis/name-resolution-proofs/array-source-scalar-element-storage.md
    // Native proof: naming.array-source.combined-read-enumeration
    // docs/design/analysis/name-resolution-proofs/array-source-combined-read-enumeration.md
    // Native proof: naming.array-source.combined-unset
    // docs/design/analysis/name-resolution-proofs/array-source-combined-unset.md
    let registry = crate::CommandRegistry::build_default();
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
        let dialect = crate::InvocationDialect::of_profile(
            crate::model::ingress::resolve_known_environment(profile)
                .unwrap()
                .unit_profile(),
        );
        let query = dialect.authoring_query();
        for (head, arguments, ordinal) in [
            ("set", vec!["a(b)", "VALUE"], 0),
            ("array", vec!["set", "a(b)", "k V"], 1),
            ("array", vec!["get", "a(b)"], 1),
            ("array", vec!["names", "a(b)"], 1),
            ("array", vec!["unset", "a(b)"], 1),
        ] {
            let selected = registry
                .resolve_invocation(head, &arguments, query)
                .unwrap();
            assert_eq!(
                selected.facts().variable_receiver_operand_form(ordinal),
                Some(VariableReceiverOperandForm::Combined),
                "{head} {arguments:?}"
            );
            assert!(
                selected
                    .facts()
                    .variable_receiver_operand_form(ordinal + 1)
                    .is_none()
            );
        }
    }
    let query = Some(SurfaceQuery::core(tcl_dialect::model::Family::Tcl, "8.6"));
    let global = registry
        .resolve_invocation("global", &["x"], query)
        .unwrap();
    assert!(global.facts().variable_receiver_operand_form(0).is_none());
    let array_get = registry
        .resolve_invocation("array", &["get", "a(b)"], query)
        .unwrap();
    assert!(
        array_get.facts().successful_handler.is_none(),
        "receiver metadata cannot donate a normal handler"
    );
    let mut explicit_withdrawal = array_get.facts();
    explicit_withdrawal.variable_receivers = Some(&[]);
    assert!(
        explicit_withdrawal
            .variable_receiver_operand_form(1)
            .is_none()
    );
    let wrong_arity = registry
        .resolve_invocation("array", &["get"], query)
        .unwrap();
    assert!(
        wrong_arity
            .facts()
            .variable_receiver_operand_form(1)
            .is_none()
    );
    let mut only_role = registry
        .resolve_invocation("set", &["x", "V"], query)
        .unwrap()
        .facts();
    only_role.native_compilation = None;
    only_role.successful_handler = None;
    assert!(
        only_role.variable_receiver_operand_form(0).is_none(),
        "a role alone cannot select native receiver semantics"
    );
}

#[test]
fn authored_receiver_form_does_not_require_native_procedure_acceptance() {
    // Implementation contract: naming.vendor.original-registry-metadata
    // docs/design/analysis/name-resolution-proofs/vendor-original-registry-metadata.md
    use crate::InvocationWord::{Dynamic, Expanded, Literal, Opaque};
    use VariableReceiverOperandForm::Combined;
    let mut registry = crate::CommandRegistry::build_default();
    registry.insert(CommandSpec {
        name: "authored-definer",
        arity: Arity::exact(3),
        arg_roles: &[
            (0, ArgRole::VarWrite),
            (1, ArgRole::ParamList),
            (2, ArgRole::Body),
        ],
        variable_receivers: Some(&[(0, Combined)]),
        procedure_definition: Some(crate::native_procedure::NativeProcedureDefinitionSpec::Core),
        ..CommandSpec::DEFAULT
    });
    for (arguments, expected) in [
        (vec![Dynamic, Dynamic, Literal("")], Some(Combined)),
        (vec![Dynamic, Dynamic], None),
        (vec![Dynamic, Expanded, Literal("")], None),
        (vec![Opaque, Dynamic, Literal("")], None),
    ] {
        let selected = registry
            .resolve_structured_invocation(
                crate::InvocationWords::structured(Literal("authored-definer"), &arguments),
                None,
            )
            .resolved()
            .unwrap();
        assert_eq!(
            selected.authored_source_variable_receiver_operand_form(0),
            expected
        );
        assert!(
            selected
                .authored_source_variable_receiver_operand_form(1)
                .is_none()
        );
        assert!(!selected.facts().arg_roles_complete);
        assert!(selected.facts().variable_receiver_operand_form(0).is_none());
        assert!(selected.facts().successful_handler.is_none());
        assert!(selected.facts().native_compilation.is_none());
    }
    registry.insert(CommandSpec {
        name: "role-only",
        arity: Arity::exact(1),
        arg_roles: &[(0, ArgRole::VarWrite)],
        ..CommandSpec::DEFAULT
    });
    let selected = registry
        .resolve_invocation("role-only", &["x"], None)
        .unwrap();
    assert!(selected.authored_source_argument_roles().1);
    assert!(
        selected
            .authored_source_variable_receiver_operand_form(0)
            .is_none()
    );
}

#[test]
// Implementation contract: naming.variable.trace-source-receiver-purpose
// docs/design/analysis/name-resolution-proofs/trace-source-receiver-purpose.md
fn original_trace_receiver_forms_select_variable_grammar_ordinals_and_release() {
    use VariableReceiverOperandForm::TraceSubject;
    let registry = crate::CommandRegistry::build_default();
    for version in tcl_dialect::TclVersion::ALL {
        let dialect = crate::InvocationDialect::for_version(version);
        for arguments in [
            vec!["add", "variable", "::v(k)", "read", "callback"],
            vec!["remove", "var", "::v(k)", "read", "callback"],
            vec!["info", "variable", "::v(k)"],
        ] {
            let invocation = registry
                .resolve_invocation("trace", &arguments, dialect.authoring_query())
                .unwrap();
            assert_eq!(
                invocation.authored_source_variable_receiver_operand_form(2),
                Some(TraceSubject),
                "{version:?}: {arguments:?}"
            );
            assert_eq!(
                invocation.facts().variable_receiver_operand_form(2),
                Some(TraceSubject)
            );
            assert!(
                invocation
                    .authored_source_variable_receiver_operand_form(1)
                    .is_none()
            );
        }
        for arguments in [
            vec!["add", "command", "set", "rename", "callback"],
            vec!["remove", "execution", "set", "enter", "callback"],
            vec!["info", "command", "set"],
            vec!["info", "execution", "set"],
            vec!["info", "var"],
        ] {
            let invocation = registry
                .resolve_invocation("trace", &arguments, dialect.authoring_query())
                .unwrap();
            assert!(
                invocation
                    .authored_source_variable_receiver_operand_form(2)
                    .is_none(),
                "{version:?}: {arguments:?}"
            );
        }
        if version < tcl_dialect::TclVersion::V9_0 {
            for arguments in [
                vec!["variable", "::v(k)", "r", "callback"],
                vec!["vdelete", "::v(k)", "r", "callback"],
                vec!["vinfo", "::v(k)"],
            ] {
                let invocation = registry
                    .resolve_invocation("trace", &arguments, dialect.authoring_query())
                    .unwrap();
                assert_eq!(
                    invocation.authored_source_variable_receiver_operand_form(1),
                    Some(TraceSubject),
                    "{version:?}: {arguments:?}"
                );
                assert!(
                    invocation
                        .authored_source_variable_receiver_operand_form(0)
                        .is_none()
                );
            }
        } else if let Some(invocation) =
            registry.resolve_invocation("trace", &["vinfo", "::v"], dialect.authoring_query())
        {
            assert!(
                invocation
                    .authored_source_variable_receiver_operand_form(1)
                    .is_none()
            );
        }
        for (subject, expected) in [
            (crate::InvocationWord::Dynamic, Some(TraceSubject)),
            (
                crate::InvocationWord::KnownBytes(b"v\xff"),
                Some(TraceSubject),
            ),
            (crate::InvocationWord::Expanded, None),
        ] {
            let words = [
                crate::InvocationWord::Literal("info"),
                crate::InvocationWord::Literal("variable"),
                subject,
            ];
            let invocation = registry
                .resolve_structured_invocation(
                    crate::InvocationWords::structured(
                        crate::InvocationWord::Literal("trace"),
                        &words,
                    ),
                    dialect.authoring_query(),
                )
                .resolved()
                .unwrap();
            assert_eq!(
                invocation.authored_source_variable_receiver_operand_form(2),
                expected
            );
        }
        let dynamic = registry
            .resolve_structured_invocation(
                crate::InvocationWords::structured(
                    crate::InvocationWord::Literal("trace"),
                    &[
                        crate::InvocationWord::Literal("info"),
                        crate::InvocationWord::Dynamic,
                        crate::InvocationWord::Literal("::v"),
                    ],
                ),
                dialect.authoring_query(),
            )
            .resolved()
            .unwrap();
        assert!(
            dynamic
                .authored_source_variable_receiver_operand_form(2)
                .is_none()
        );
    }
}

#[test]
fn original_trace_subject_extent_preserves_counted_original_and_independent_array_form() {
    // Native proof: naming.variable.trace-subject-counted-zero-address
    // docs/design/analysis/name-resolution-proofs/trace-subject-counted-zero-address.md
    use tcl_syntax::naming::{NativeNameProtocol, NativeVariableInputForm};
    for version in tcl_dialect::TclVersion::ALL {
        let protocol = NativeNameProtocol::C(version);
        let NativeVariableInputForm::Combined(selected) = VariableReceiverOperandForm::TraceSubject
            .input_form(protocol, b"::v\0tail(k)")
            .unwrap()
        else {
            panic!("trace subject is combined after its selected ingress");
        };
        assert_eq!(selected, b"::v");
        assert!(
            protocol
                .combined_variable_input(selected)
                .element()
                .is_none()
        );
        let NativeVariableInputForm::Combined(selected) = VariableReceiverOperandForm::TraceSubject
            .input_form(protocol, b"::v\xc0\x80tail(k)")
            .unwrap()
        else {
            panic!("trace subject");
        };
        let name = protocol.combined_variable_input(selected);
        assert_eq!(name.root().selected(), b"::v\xc0\x80tail");
        assert_eq!(name.element().unwrap().selected(), b"k");
    }
    assert!(
        VariableReceiverOperandForm::TraceSubject
            .input_form(NativeNameProtocol::Jim084, b"::v")
            .is_none()
    );
}

#[cfg(test)]
mod original_source_presentation_tests {
    #[test]
    fn original_source_presentation_uses_selected_roles_and_effective_ordinals() {
        // naming.core.original-command-source-schema
        // docs/design/analysis/name-resolution-proofs/original-command-source-schema.md
        use crate::{ArgPresentation, InvocationWord};
        let context = crate::model::ingress::static_context_for("tcl8.6");
        let arguments = [
            InvocationWord::Literal("set i 0"),
            InvocationWord::Dynamic,
            InvocationWord::Literal("incr i"),
            InvocationWord::Literal("puts $i"),
        ];
        let resolution = crate::model::assembly::resolve_structured_invocation_in_resolved_context(
            context.commands(),
            Some(context.context()),
            crate::InvocationWords::structured(InvocationWord::Literal("for"), &arguments),
            tcl_dialect::model::InvocationRealm::RuleLoader,
        );
        let selected = resolution.resolved().unwrap();
        assert_eq!(
            selected.authored_source_argument_presentation(0),
            Some(ArgPresentation::InlineScript)
        );
        assert_eq!(
            selected.authored_source_argument_presentation(2),
            Some(ArgPresentation::InlineScript)
        );
        assert_eq!(
            selected.authored_source_argument_presentation(3),
            Some(ArgPresentation::BlockScript)
        );
        assert_eq!(selected.authored_source_argument_presentation(1), None);
        assert_eq!(selected.authored_source_argument_presentation(4), None);
        let expanded = [InvocationWord::Expanded];
        let uncertain = crate::model::assembly::resolve_structured_invocation_in_resolved_context(
            context.commands(),
            Some(context.context()),
            crate::InvocationWords::structured(InvocationWord::Literal("for"), &expanded),
            tcl_dialect::model::InvocationRealm::RuleLoader,
        )
        .resolved()
        .unwrap();
        assert_eq!(uncertain.authored_source_argument_presentation(0), None);
    }
    #[test]
    fn selected_option_keyword_table_retains_prefix_positions_and_values() {
        // naming.minifier.complete-logical-metadata
        // docs/design/analysis/name-resolution-proofs/minifier-complete-logical-metadata.md
        let context = crate::model::ingress::static_context_for("tcl8.6");
        let arguments = [
            crate::InvocationWord::Literal("equal"),
            crate::InvocationWord::Literal("-length"),
            crate::InvocationWord::Literal("-nocase"),
            crate::InvocationWord::Literal("A"),
            crate::InvocationWord::Literal("b"),
        ];
        let resolution = crate::model::assembly::resolve_structured_invocation_in_resolved_context(
            context.commands(),
            Some(context.context()),
            crate::InvocationWords::structured(
                crate::InvocationWord::Literal("string"),
                &arguments,
            )
            .with_profile(context.commands().profile()),
            tcl_dialect::model::InvocationRealm::RuleLoader,
        );
        let selected = resolution.resolved().unwrap();
        let options = selected.semantics.options;
        let table = options.keyword_table();
        assert_eq!(table.resolve("-n").unique(), Some("-nocase"));
        let occurrences = options
            .prefix_occurrences(
                selected
                    .words
                    .arguments()
                    .slice_from(selected.semantics.argument_offset),
            )
            .unwrap();
        assert_eq!(occurrences.len(), 1);
        assert_eq!(occurrences[0].option.unwrap().name, "-length");
        assert_eq!(occurrences[0].values, 1..2);
    }
}

#[cfg(test)]
mod logical_binding_result_tests {
    use crate::ArgRole;
    #[test]
    fn selected_logical_result_binding_axis_keeps_native_effects_unknown() {
        // naming.minifier.logical-formal-binding-alpha
        // docs/design/analysis/name-resolution-proofs/logical-formal-binding-alpha.md
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let context = crate::model::ingress::context_for_profile(profile);
        assert!(
            crate::InvocationDialect::of_profile(profile)
                .native_name_protocol()
                .is_none()
        );
        for (arguments, expected) in [
            (Vec::new(), true),
            (vec![crate::InvocationWord::Dynamic], true),
            (
                vec![
                    crate::InvocationWord::Literal("-code"),
                    crate::InvocationWord::Literal("ok"),
                    crate::InvocationWord::Dynamic,
                ],
                false,
            ),
            (
                vec![
                    crate::InvocationWord::Literal("-errorinfo"),
                    crate::InvocationWord::Dynamic,
                ],
                false,
            ),
            (vec![crate::InvocationWord::Expanded], false),
        ] {
            let resolution =
                crate::model::assembly::resolve_structured_invocation_in_resolved_context(
                    context.commands(),
                    Some(context.context()),
                    crate::InvocationWords::structured(
                        crate::InvocationWord::Literal("return"),
                        &arguments,
                    ),
                    tcl_dialect::model::InvocationRealm::RuleLoader,
                );
            let schema = resolution.resolved().unwrap();
            assert_eq!(
                schema.authored_source_result_preserves_variable_bindings(),
                expected
            );
            if expected {
                assert_eq!(
                    schema.authored_source_argument_roles(),
                    (
                        if arguments.is_empty() {
                            Vec::new()
                        } else {
                            vec![(0, crate::ArgRole::Result)]
                        },
                        true
                    )
                );
            }
            assert!(schema.facts().effects.requires_world_barrier());
        }
    }
    #[test]
    fn structured_conditional_and_output_roles_keep_unknown_payloads() {
        // naming.source.authored-registry-role-projection
        // docs/design/analysis/name-resolution-proofs/authored-registry-role-projection.md
        use crate::InvocationWord::{Dynamic, Expanded, Literal};
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let owner = crate::model::ingress::static_context_for(profile);
            let dialect = crate::InvocationDialect::of_profile(owner.commands().profile().unwrap());
            for (head, arguments, expected) in [
                (
                    "if",
                    vec![Dynamic, Literal("then"), Dynamic],
                    vec![
                        (0, ArgRole::Expr),
                        (1, ArgRole::Keyword),
                        (2, ArgRole::Body),
                    ],
                ),
                (
                    "if",
                    vec![
                        Dynamic,
                        Literal("{}"),
                        Literal("elseif"),
                        Dynamic,
                        Literal("then"),
                        Dynamic,
                        Literal("else"),
                        Dynamic,
                    ],
                    vec![
                        (0, ArgRole::Expr),
                        (1, ArgRole::Body),
                        (2, ArgRole::Keyword),
                        (3, ArgRole::Expr),
                        (4, ArgRole::Keyword),
                        (5, ArgRole::Body),
                        (6, ArgRole::Keyword),
                        (7, ArgRole::Body),
                    ],
                ),
                ("puts", vec![Dynamic], vec![]),
                (
                    "puts",
                    vec![Literal("-nonewline"), Dynamic, Dynamic],
                    vec![(1, ArgRole::Channel)],
                ),
                (
                    "puts",
                    vec![Literal("stdout"), Dynamic],
                    vec![(0, ArgRole::Channel)],
                ),
            ] {
                let selected = owner.commands().resolve_structured_invocation(
                    crate::InvocationWords::structured(Literal(head), &arguments)
                        .with_dialect(dialect),
                    Some(owner.context().authoring_query()),
                );
                let selected = selected.resolved().unwrap();
                assert_eq!(
                    selected.authored_source_argument_roles(),
                    (expected, true),
                    "{profile} {head}"
                );
            }
            for (head, arguments) in [
                ("if", vec![Dynamic, Dynamic]),
                ("if", vec![Dynamic, Literal("{}"), Dynamic]),
                ("if", vec![Expanded, Literal("{}")]),
                ("puts", vec![Dynamic, Dynamic]),
                ("puts", vec![Expanded]),
            ] {
                let selected = owner.commands().resolve_structured_invocation(
                    crate::InvocationWords::structured(Literal(head), &arguments)
                        .with_dialect(dialect),
                    Some(owner.context().authoring_query()),
                );
                assert!(
                    !selected
                        .resolved()
                        .unwrap()
                        .authored_source_argument_roles()
                        .1,
                    "{profile} {head}"
                );
            }
        }
    }
}

#[cfg(test)]
mod logical_frame_source_role_tests {
    use crate::value_transfer::OperandId;
    use crate::{
        ArgRole, CommandRegistry, FrameLevel, InvocationArguments, InvocationDialect,
        InvocationResolutionUnresolved, InvocationWord, InvocationWords, TclType,
    };

    #[test]
    fn logical_frame_source_roles_use_consensus_without_native_frame_facts() {
        // naming.source.original-produced-command-prefix
        // docs/design/analysis/name-resolution-proofs/original-produced-command-prefix.md
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let context = crate::model::ingress::context_for_profile(profile);
        let dialect = InvocationDialect::of_profile(profile);
        assert!(dialect.native_name_protocol().is_none());
        for (head, arguments, roles) in [
            (
                "upvar",
                vec![
                    InvocationWord::Literal("#0"),
                    InvocationWord::Literal("original"),
                    InvocationWord::Literal("local"),
                ],
                vec![(2, ArgRole::VarWrite)],
            ),
            (
                "uplevel",
                vec![InvocationWord::Literal("#0"), InvocationWord::Dynamic],
                vec![(1, ArgRole::Body)],
            ),
        ] {
            let resolution =
                crate::model::assembly::resolve_structured_invocation_in_resolved_context(
                    context.commands(),
                    Some(context.context()),
                    InvocationWords::structured(InvocationWord::Literal(head), &arguments)
                        .with_dialect(dialect),
                    tcl_dialect::model::InvocationRealm::RuleLoader,
                );
            let schema = resolution.resolved().unwrap();
            assert_eq!(
                schema.authored_logical_source_argument_roles(),
                (roles, true)
            );
            assert_eq!(
                schema.authored_logical_source_script_arguments(),
                Some(if head == "uplevel" {
                    vec![1]
                } else {
                    Vec::new()
                }),
            );
            assert_eq!(
                schema.authored_logical_source_plain_script_arguments(),
                schema.authored_logical_source_script_arguments(),
            );
            assert!(schema.authored_source_script_arguments().is_none());
            assert!(schema.authored_source_plain_script_arguments().is_none());
            assert!(!schema.authored_source_argument_roles().1);
            let facts = schema.facts();
            assert!(!facts.arg_roles_complete);
            assert_eq!(
                facts
                    .frame_effect
                    .unwrap()
                    .resolve_arguments(schema.words.arguments()),
                crate::frame_effect::FrameArgumentResolution::Unknown,
            );
            if head == "uplevel" {
                assert!(facts.effects.requires_world_barrier());
            } else {
                // A callback barrier is not an unknown alias target. The
                // registry retains the unresolved cell/trace transition;
                // Logical source roles do not install a native frame link.
                let transitions = facts.state_transitions.declared().unwrap();
                assert_eq!(transitions.facts().len(), 1);
                assert!(matches!(
                    &transitions.facts()[0].transition,
                    crate::StateTransition::Widen(widening)
                        if widening.domains == [
                            crate::StateTransitionDomain::VariableCells,
                            crate::StateTransitionDomain::VariableTraces,
                        ]
                ));
            }
        }
    }

    #[test]
    fn logical_frame_source_roles_keep_dynamic_divergent_and_expanded_layouts_unknown() {
        // naming.source.original-produced-command-prefix
        // docs/design/analysis/name-resolution-proofs/original-produced-command-prefix.md
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let context = crate::model::ingress::context_for_profile(profile);
        for arguments in [
            vec![
                InvocationWord::Dynamic,
                InvocationWord::Literal("original"),
                InvocationWord::Literal("local"),
            ],
            vec![
                InvocationWord::Literal("+1"),
                InvocationWord::Literal("original"),
                InvocationWord::Literal("local"),
            ],
            vec![InvocationWord::Expanded],
        ] {
            let resolution =
                crate::model::assembly::resolve_structured_invocation_in_resolved_context(
                    context.commands(),
                    Some(context.context()),
                    InvocationWords::structured(InvocationWord::Literal("upvar"), &arguments)
                        .with_dialect(InvocationDialect::of_profile(profile)),
                    tcl_dialect::model::InvocationRealm::RuleLoader,
                );
            let schema = resolution.resolved().unwrap();
            assert_eq!(
                schema.authored_logical_source_argument_roles(),
                (Vec::new(), false)
            );
            assert!(!schema.facts().arg_roles_complete);
        }
    }

    /// The context a derived-query test resolves under: `profile`'s.
    fn derived_context(profile: &str) -> crate::value_transfer::AnalysisContext {
        crate::value_transfer::AnalysisContext::detached(Some(
            tcl_dialect::DialectProfile::find(profile).expect("a catalogued profile"),
        ))
    }

    /// The derived-query layer on a literal call: every query answers, under
    /// the query the invocation resolved at — the one the context fixes.
    #[test]
    fn derived_queries_answer_a_literal_call() {
        let context = derived_context("tcl9.0");
        let registry = CommandRegistry::build_default();
        let resolve = |name, args: &'static [&'static str]| {
            registry
                .invocation(InvocationWords::literals(name, args), &context)
                .resolved()
                .expect("a shipped command resolves")
        };

        let array_for = resolve("array", &["for", "{k v}", "a", "{body}"]);
        assert_eq!(array_for.dialect, context.surface_query());
        let plan = array_for.clause_plan().expect("`array for` walks at 9.0");
        assert!(plan.roles.contains(&(3, ArgRole::Body)), "{plan:?}");

        let subst = resolve("subst", &["-nocommands", "$x"]);
        let effects = subst.option_effects();
        assert!(effects.complete);
        assert_eq!(effects.option_end, 1);
        let kinds = subst.substitutions_performed().expect("subst substitutes");
        assert!(kinds.variables && kinds.backslashes && !kinds.commands);

        let upvar = resolve("upvar", &["1", "a", "b"]);
        let roles = upvar.arg_roles().expect("a literal layout");
        assert!(roles.contains(&(2, ArgRole::VarWrite)), "{roles:?}");
        assert_eq!(
            upvar.frame_effect(),
            Some((FrameLevel::Relative(1), vec![OperandId(1), OperandId(2)]))
        );

        let lsearch = resolve("lsearch", &["-regexp", "$list", "a.*"]);
        let patterns = lsearch.pattern_args();
        assert_eq!(patterns.len(), 1, "{patterns:?}");
        assert_eq!(patterns[0].index, 2);

        let switch = resolve("switch", &["-glob", "x", "a*", "body"]);
        let (case, clauses) = switch.case_invocation().expect("a case list");
        assert_eq!(case.subject_index, Some(1));
        assert_eq!(case.mode, crate::spec::CaseMatchMode::Glob);
        assert_eq!(clauses.len(), 1);
        assert_eq!(clauses[0].body_index, Some(3));

        assert_eq!(
            resolve("string", &["length", "abc"]).return_type(),
            Some(TclType::Int)
        );
        assert!(!resolve("set", &["x", "1"]).effects().accesses().is_empty());
    }

    /// Two resolutions no query can answer from: a computed head resolves
    /// nothing, and neither does a command the context's release lacks.
    #[test]
    fn derived_queries_abstain_on_a_computed_head_and_an_absent_command() {
        let registry = CommandRegistry::build_default();
        let context = derived_context("tcl8.6");
        let arguments = [crate::InvocationWord::Literal("x")];
        let computed_head = registry.invocation(
            InvocationWords::structured(crate::InvocationWord::Dynamic, &arguments),
            &context,
        );
        assert!(computed_head.resolved().is_none());
        assert!(matches!(
            computed_head.unresolved(),
            Some(InvocationResolutionUnresolved::ComputedHead { .. })
        ));
        let absent = registry.invocation(InvocationWords::literals("lpop", &["l"]), &context);
        assert!(absent.resolved().is_none(), "`lpop` is Tcl 9.0");
        assert!(matches!(
            absent.unresolved(),
            Some(InvocationResolutionUnresolved::UnknownLiteralHead { spelling: "lpop" })
        ));
    }

    /// Each query abstains rather than answering for words it cannot read:
    /// an expansion abstains the role table, the frame and a return-type
    /// hook; a computed word where an option could stand abstains the role
    /// table and the case-list reading; a computed word where none can is an
    /// operand like any other.
    #[test]
    fn derived_queries_abstain_on_words_they_cannot_read() {
        use crate::InvocationWord::{Dynamic, Expanded, Literal};
        let registry = CommandRegistry::build_default();
        let context = derived_context("tcl8.6");
        let resolve = |head: &'static str, arguments: &'static [crate::InvocationWord<'static>]| {
            registry
                .invocation(
                    InvocationWords::structured(Literal(head), arguments),
                    &context,
                )
                .resolved()
                .expect("a shipped command resolves")
        };

        let upvar = resolve("upvar", &[Expanded, Literal("b")]);
        assert_eq!(upvar.arg_roles(), None);
        assert_eq!(upvar.frame_effect(), None);
        assert_eq!(
            resolve("regexp", &[Expanded, Literal("s")]).return_type(),
            None
        );

        let switch = resolve("switch", &[Dynamic, Literal("x"), Literal("a {}")]);
        assert_eq!(switch.case_invocation(), None);
        assert_eq!(switch.arg_roles(), None);

        let switch = resolve("switch", &[Dynamic, Literal("a {}")]);
        let (case, _) = switch.case_invocation().expect("the reading holds");
        assert_eq!(case.subject_index, Some(0));
        assert_eq!(case.clause_list_index, Some(1));
        // A computed pattern is a pattern: `switch` has no clause flags.
        let switch = resolve("switch", &[Literal("x"), Dynamic, Literal("{b}")]);
        let (_, clauses) = switch.case_invocation().expect("the reading holds");
        assert_eq!(clauses[0].pattern_index, 1);

        let uplevel = resolve("uplevel", &[Dynamic, Literal("{set x 1}")]);
        assert_eq!(
            uplevel.frame_effect(),
            Some((FrameLevel::Dynamic, vec![OperandId(1)]))
        );
        let uplevel = resolve("uplevel", &[Dynamic]);
        assert_eq!(
            uplevel.frame_effect(),
            Some((FrameLevel::DEFAULT, vec![OperandId(0)])),
            "a lone computed word is the script"
        );
    }

    /// Every answer is keyed on the context: the same words answer
    /// differently under releases that read them differently.
    #[test]
    fn derived_queries_answer_under_the_context_release() {
        let registry = CommandRegistry::build_default();
        let at = |profile: &str, name: &'static str, args: &'static [&'static str]| {
            let context = derived_context(profile);
            let invocation = registry
                .invocation(InvocationWords::literals(name, args), &context)
                .resolved()
                .expect("a shipped command resolves");
            (
                invocation.clause_plan().is_some(),
                invocation.frame_effect().map(|(level, _)| level),
                invocation.substitutions_performed(),
            )
        };
        // `array for` is Tcl 9.0.
        assert!(!at("tcl8.6", "array", &["for", "{k v}", "a", "{}"]).0);
        assert!(at("tcl9.0", "array", &["for", "{k v}", "a", "{}"]).0);
        // A leading-zero level is octal on 8.6 and decimal on 9.0.
        assert_eq!(
            at("tcl8.6", "upvar", &["010", "a", "b"]).1,
            Some(FrameLevel::Relative(8))
        );
        assert_eq!(
            at("tcl9.0", "upvar", &["010", "a", "b"]).1,
            Some(FrameLevel::Relative(10))
        );
        // `subst`'s positive switches are Tcl 9.1: below it the call is
        // unreadable and every kind runs.
        let positive = at("tcl9.1", "subst", &["-variables", "$x"])
            .2
            .expect("subst substitutes");
        assert!(positive.variables && !positive.commands && !positive.backslashes);
        let below = at("tcl9.0", "subst", &["-variables", "$x"])
            .2
            .expect("subst substitutes");
        assert!(below.variables && below.commands && below.backslashes);
    }

    /// The one place the derived queries and the by-name answers part: the
    /// resolution selects its subcommand under its release, so a prefix the
    /// release makes unique selects it — `array d` is `donesearch` at 8.6,
    /// where a release-blind lookup finds it ambiguous with 9.0's `default`.
    #[test]
    fn a_prefix_the_release_makes_unique_selects_its_subcommand() {
        let registry = crate::model::ingress::static_context_for("tcl8.6").commands();
        let invocation = registry
            .invocation(
                InvocationWords::literals("array", &["d", "a", "s"]),
                &derived_context("tcl8.6"),
            )
            .resolved()
            .expect("`array` resolves");
        assert_eq!(
            invocation
                .subcommand
                .resolved()
                .map(|sub| sub.canonical_name),
            Some("donesearch")
        );
        let roles = invocation.arg_roles().expect("a literal layout");
        assert!(roles.contains(&(1, ArgRole::VarRead)), "{roles:?}");
        assert!(
            registry
                .arg_indices_for_role("array", &["d", "a", "s"], ArgRole::VarRead)
                .is_empty(),
            "the release-blind lookup finds `d` ambiguous"
        );
    }

    /// Calls whose layout a computed word decides, or leaves decided: the
    /// corpus the derived role table is held to the by-name one on.
    const COMPUTED_WORD_CORPUS: &[(&str, &[crate::InvocationWord<'static>])] = {
        use crate::InvocationWord::{Dynamic, DynamicNonOption, Expanded, Literal};
        &[
            ("upvar", &[Dynamic, Literal("a"), Literal("b")]),
            ("upvar", &[Literal("1"), Dynamic, Literal("b"), Dynamic]),
            ("switch", &[Dynamic, Literal("{a {b}}")]),
            ("switch", &[Dynamic, Literal("x"), Literal("{a b}")]),
            (
                "switch",
                &[Literal("-glob"), Literal("--"), Dynamic, Literal("{a b}")],
            ),
            (
                "switch",
                &[DynamicNonOption, Literal("x"), Literal("{a b}")],
            ),
            ("case", &[Dynamic, Literal("in"), Literal("{a {b}}")]),
            ("lsearch", &[Dynamic, Literal("l"), Literal("a*")]),
            ("lsearch", &[Literal("-glob"), Dynamic, Dynamic]),
            ("regexp", &[Dynamic, Literal("s"), Literal("m")]),
            ("regexp", &[Literal("-inline"), Dynamic, Dynamic]),
            ("foreach", &[Dynamic, Dynamic, Literal("{body}")]),
            ("foreach", &[Literal("{a b}"), Dynamic, Literal("{body}")]),
            (
                "if",
                &[Dynamic, Literal("{a}"), Literal("else"), Literal("{b}")],
            ),
            ("if", &[Literal("{$c}"), Dynamic, Literal("{a}")]),
            (
                "try",
                &[
                    Literal("{a}"),
                    Literal("on"),
                    Literal("error"),
                    Dynamic,
                    Literal("{b}"),
                ],
            ),
            (
                "try",
                &[
                    Literal("{a}"),
                    Dynamic,
                    Literal("error"),
                    Literal("m"),
                    Literal("{b}"),
                ],
            ),
            (
                "dict",
                &[Literal("for"), Literal("{k v}"), Dynamic, Literal("{body}")],
            ),
            ("dict", &[Dynamic, Literal("d")]),
            ("string", &[Dynamic, Literal("abc")]),
            ("lsort", &[Literal("-command"), Dynamic, Dynamic]),
            ("after", &[Literal("100"), Dynamic]),
            ("format", &[Dynamic, Literal("a")]),
            ("scan", &[Dynamic, Literal("%d"), Literal("v")]),
            ("set", &[Expanded]),
            ("namespace", &[Literal("eval"), Dynamic, Literal("{body}")]),
            (
                "trace",
                &[
                    Literal("add"),
                    Literal("variable"),
                    Literal("x"),
                    Literal("write"),
                    Dynamic,
                ],
            ),
            ("lassign", &[Dynamic, Literal("a"), Literal("b")]),
            (
                "interp",
                &[
                    Literal("alias"),
                    Literal("{}"),
                    Literal("a"),
                    Literal("{}"),
                    Dynamic,
                ],
            ),
        ]
    };

    /// `arg_roles` is `arg_indices_for_role_words` over every role, re-keyed
    /// on the resolution — on computed words too, where the layout proof
    /// decides between a position and an abstention.
    #[test]
    fn arg_roles_agree_with_the_registry_role_answer_on_computed_words() {
        use crate::InvocationWord::Literal;
        let corpus = COMPUTED_WORD_CORPUS;
        for (profile, &(name, arguments)) in ["tcl8.6", "tcl9.0"]
            .into_iter()
            .flat_map(|profile| corpus.iter().map(move |row| (profile, row)))
        {
            let registry = crate::model::ingress::static_context_for(profile).commands();
            let context = derived_context(profile);
            let Some(invocation) = registry
                .invocation(
                    InvocationWords::structured(Literal(name), arguments),
                    &context,
                )
                .resolved()
            else {
                assert_eq!(
                    name, "case",
                    "{profile}: only `case` is absent (Tcl 9 dropped it)"
                );
                continue;
            };
            let words = InvocationArguments::structured(arguments);
            let expected = ArgRole::ALL
                .iter()
                .map(|&role| {
                    registry
                        .arg_indices_for_role_words(name, words, role)
                        .map(|indices| indices.into_iter().map(move |index| (index, role)))
                })
                .collect::<Option<Vec<_>>>()
                .map(|per_role| {
                    let mut roles: Vec<(usize, ArgRole)> = per_role.into_iter().flatten().collect();
                    roles.sort_by_key(|&(index, _)| index);
                    roles.dedup();
                    roles
                });
            assert_eq!(
                invocation.arg_roles(),
                expected,
                "{profile} {name} {arguments:?}: the derived table"
            );
            assert_eq!(
                invocation.pattern_args(),
                registry.pattern_args_words(name, words),
                "{profile} {name} {arguments:?}: the pattern answer"
            );
        }
    }

    /// The return-type hooks read a computed word as its source spelling
    /// would read — a value they cannot see — so the query answers what
    /// `return_type_for_call` answers over the source text.
    #[test]
    fn return_type_reads_a_computed_word_as_its_source_spelling_reads() {
        use crate::InvocationWord::{Dynamic, Literal};
        let registry = CommandRegistry::build_default();
        let context = derived_context("tcl9.0");
        let corpus: &[(&str, &[crate::InvocationWord<'static>], &[&str])] = &[
            ("lsearch", &[Dynamic, Dynamic], &["$l", "$p"]),
            (
                "lsearch",
                &[Literal("-all"), Dynamic, Dynamic],
                &["-all", "$l", "$p"],
            ),
            (
                "regexp",
                &[Dynamic, Literal("a"), Literal("b")],
                &["$opt", "a", "b"],
            ),
            ("regexp", &[Literal("-about"), Dynamic], &["-about", "$re"]),
            (
                "regexp",
                &[Literal("--"), Dynamic, Dynamic],
                &["--", "$re", "$s"],
            ),
            (
                "scan",
                &[Dynamic, Literal("%d"), Literal("v")],
                &["$s", "%d", "v"],
            ),
            (
                "regsub",
                &[Dynamic, Dynamic, Dynamic],
                &["$re", "$s", "$sub"],
            ),
        ];
        for &(name, arguments, source) in corpus {
            let invocation = registry
                .invocation(
                    InvocationWords::structured(Literal(name), arguments),
                    &context,
                )
                .resolved()
                .expect("a shipped command resolves");
            let spec = registry.get(name).expect("a shipped command");
            assert_eq!(
                invocation.return_type(),
                spec.return_type_for_call(source),
                "{name} {source:?}"
            );
        }
    }
}

impl ResolvedInvocation<'_, '_> {
    /// Original effective operands of a selected conditional index schema.
    /// Unknown selectors, expansions and incompatible shapes supply no advice.
    /// Selection does not establish a native handler or substituted value.
    #[must_use]
    pub fn authored_source_index_bounds(&self) -> Option<crate::SourceIndexBoundsInvocation> {
        // naming.diagnostic.registry-source-ownership
        // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
        let selected = self.semantics.script_metadata;
        let declared = selected
            .subcommand
            .map_or(selected.command.source_index_bounds, |sub| {
                sub.source_index_bounds
            });
        let operation = declared
            .or_else(|| crate::SourceIndexBounds::from_operation(self.semantics.operation))?;
        let count = self.words.arguments().exact_argv_len()?;
        let first = self.semantics.argument_offset;
        let local_count = count.checked_sub(first)?;
        let arity = self.authored_source_arity()?;
        if arity.count.indeterminate
            || !arity.arity.accepts(arity.count.minimum)
            || !operation.accepts_argument_count(local_count)
        {
            return None;
        }
        Some(crate::SourceIndexBoundsInvocation {
            operation,
            arguments: first..count,
            dialect: self.words.arguments().dialect(),
        })
    }

    /// Select the actual authored source path operation and complete effective
    /// post-head argument range. Bound prefix values retain their ordinals;
    /// unknown expansion/selectors and unmatched forms supply no algebra.
    /// This is conditional source advice, not a native handler or value.
    #[must_use]
    pub fn authored_source_path_operation(
        &self,
    ) -> Option<crate::source_path::SourcePathInvocation> {
        // naming.navigation.retained-path-source-inventory
        // docs/design/analysis/name-resolution-proofs/retained-path-source-inventory.md
        if self.form.is_some() {
            return None;
        }
        let operation = self
            .semantics
            .script_metadata
            .subcommand?
            .source_path_operation?;
        let count = self.words.arguments().exact_argv_len()?;
        let first = self.semantics.argument_offset;
        let values = count.checked_sub(first)?;
        operation.accepts_argument_count(values).then_some(
            crate::source_path::SourcePathInvocation {
                operation,
                arguments: first..count,
            },
        )
    }
}
