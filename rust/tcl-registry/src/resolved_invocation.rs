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
use crate::frame_effect::FrameEffectSpec;
use crate::hooks::{CodegenHookId, InlineCodegenHookId, LoweringHookId};
use crate::hover::OptionSpec;
use crate::intrinsic::IntrinsicId;
use crate::invocation_words::{InvocationWordKind, InvocationWords};
use crate::literal_validation::{LiteralArgumentValidation, LiteralArgumentValidator};
use crate::representation::RepresentationEffect;
use crate::result_stability::ResultStability;
use crate::semantic_operation::SemanticOperationId;
use crate::side_effects::SideEffect;
use crate::spec::{
    ArgRoleCountResolver, ArgRoleLayoutResolver, ArgRoleResolver, ArgRoleResolverInput,
    CommandSpec, SubCommand,
};
use crate::state_transition::{
    ResolvedStateTransitions, StateTransitionKnowledge, StateTransitions,
};
use crate::traits::Traits;
use crate::types::{ReturnElements, TclType, VarElementsEffect, VarWriteTyping};
use crate::world_effect::TransitionEffectCoverages;
use crate::world_effect::{EffectFootprint, ResolvedWorldEffects};
use tcl_dialect::model::{SpecSurface, SurfaceQuery};

pub(crate) fn descriptor_operation(
    semantic: Option<SemanticOperationId>,
    lowering: Option<LoweringHookId>,
    codegen: Option<CodegenHookId>,
    inline_codegen: Option<InlineCodegenHookId>,
) -> Option<SemanticOperationId> {
    semantic
        .or(lowering.map(SemanticOperationId::StructuredLowering))
        .or_else(|| {
            inline_codegen
                .and_then(IntrinsicId::from_legacy_inline_codegen)
                .or_else(|| codegen.and_then(IntrinsicId::from_legacy_codegen))
                .map(SemanticOperationId::Intrinsic)
        })
}

fn resolved_operation(
    spec: &CommandSpec,
    sub: Option<&SubCommand>,
    form: Option<&CommandForm>,
) -> SemanticOperationId {
    form.and_then(|form| {
        descriptor_operation(
            form.semantic_operation,
            form.lowering_hook,
            form.codegen_hook,
            None,
        )
    })
    .or_else(|| {
        sub.and_then(|sub| {
            descriptor_operation(
                sub.semantic_operation,
                sub.lowering_hook,
                sub.codegen_hook,
                sub.inline_codegen_hook,
            )
        })
    })
    .or_else(|| {
        descriptor_operation(
            spec.semantic_operation,
            spec.lowering_hook,
            spec.codegen_hook,
            spec.inline_codegen_hook,
        )
    })
    .unwrap_or(SemanticOperationId::Invoke)
}

// This is the single exhaustive projection from three nested registry owners
// into one semantic view; splitting it would duplicate the inheritance rules.
#[allow(clippy::too_many_lines)]
fn resolve_invocation_semantics<'r>(
    spec: &'r CommandSpec,
    sub: Option<&'r SubCommand>,
    form: Option<&'r CommandForm>,
    inherit_command: bool,
) -> InvocationSemantics<'r> {
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
        operation: if inherit_command {
            resolved_operation(spec, sub, form)
        } else {
            form.and_then(|form| {
                descriptor_operation(
                    form.semantic_operation,
                    form.lowering_hook,
                    form.codegen_hook,
                    None,
                )
            })
            .or_else(|| {
                sub.and_then(|sub| {
                    descriptor_operation(
                        sub.semantic_operation,
                        sub.lowering_hook,
                        sub.codegen_hook,
                        sub.inline_codegen_hook,
                    )
                })
            })
            .unwrap_or(SemanticOperationId::Invoke)
        },
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
        arity: form.map_or_else(
            || sub.map_or(spec.arity, |sub| sub.arity),
            |form| form.arity,
        ),
        argument_offset: usize::from(sub.is_some()),
        arg_roles,
        arg_role_resolver,
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
pub struct InvocationOptions<'r> {
    /// Ingress-selected option availability.
    pub availability: InvocationAvailability<'r>,
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

impl<'r> InvocationOptions<'r> {
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

    /// Options admitted by the retained ingress surface and package floor.
    /// Shared and form-local rows keep their own inherited surfaces.
    pub fn available(self) -> impl Iterator<Item = &'r OptionSpec> + 'r {
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
            match self.case_list {
                Some(case) => case.option_scan_reserved_for_arguments(
                    arguments,
                    self.availability.query,
                    self.reserved_trailing_words,
                )?,
                None => self.reserved_trailing_words,
            },
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
pub struct InvocationSemantics<'r> {
    script_metadata: crate::selected_script_timing::SelectedScriptMetadata<'r>,
    /// Selected named-object factory metadata. A candidate class does not
    /// establish a live object allocation or method implementation.
    pub named_object_factory: Option<NamedObjectFactory>,
    /// Registry-selected static semantic operation identity.
    ///
    /// This is not a live command identity; runtime command binding and trace
    /// state are established by later common analyses.
    pub operation: SemanticOperationId,
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
    /// Effective command/subcommand/form option descriptors.
    pub options: InvocationOptions<'r>,
    /// Result Tcl internal-representation type, when declared.
    pub return_type: Option<TclType>,
    /// Authored native procedure definition grammar.
    pub procedure_definition: Option<crate::native_procedure::NativeProcedureDefinitionSpec>,
    /// Selected native compiler descriptor; syntax and compiler entry remain required.
    pub native_compilation: Option<crate::native_compilation::NativeCompilationSpec>,
    /// Successful handler transfer without compiler or opcode proof.
    pub successful_handler: Option<crate::native_compilation::SuccessfulHandlerSpec>,
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
    /// order.  Call [`ResolvedInvocation::effect_footprint`] to resolve this
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
}

impl InvocationSemantics<'_> {
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
    pub semantics: InvocationSemantics<'r>,
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

impl InvocationFacts {
    /// Selected deferred single-script/prefix operand positions. Unknown values
    /// keep their slots; None retains unknown layout, timing or concatenation.
    /// This metadata supplies positive navigation only, never callback absence,
    /// execution, future body admission, edit completeness or erasure permission.
    #[must_use]
    pub fn deferred_script_argument_indices(&self) -> Option<&[usize]> {
        self.deferred_script_arguments.as_deref()
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
pub(crate) fn invocation_options<'r>(
    spec: &'r CommandSpec,
    sub: Option<&'r SubCommand>,
    form: Option<&'r CommandForm>,
    availability: InvocationAvailability<'r>,
) -> InvocationOptions<'r> {
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
        reserved_trailing_words: if sub.is_none() {
            spec.reserved_trailing_words
        } else {
            0
        },
        case_list: if sub.is_none() { spec.case_list } else { None },
        base: sub.map_or(spec.options, |sub| sub.options),
        form: form.map_or(&[], |form| form.options),
    }
}

/// Assess one invocation using the authored count axis and shared option walk.
/// This count does not validate option values, effects or native compilation.
#[must_use]
pub fn count_invocation_arguments(
    arity: Arity,
    arguments: crate::InvocationArguments<'_>,
    offset: usize,
    options: InvocationOptions<'_>,
) -> Option<u16> {
    let count = arguments.exact_argv_len()?.checked_sub(offset)?;
    if arity.count == crate::arity::ArityCount::Arguments {
        return u16::try_from(count).ok();
    }
    let end = options.leading_word_count(arguments.slice_from(offset))?;
    let consumed = end.saturating_sub(options.positional_prefix_words);
    u16::try_from(count.checked_sub(consumed)?).ok()
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
        availability: InvocationAvailability<'r>,
    ) -> Self {
        let mut semantics = resolve_invocation_semantics(spec, sub, form, true);
        semantics.options = invocation_options(spec, sub, form, availability);
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
        }
    }

    pub(crate) fn new_instance(
        words: InvocationWords<'w>,
        class_spec: &'r CommandSpec,
        method: &'r SubCommand,
        form: Option<&'r CommandForm>,
        subcommand: SubcommandResolution<'w>,
        availability: InvocationAvailability<'r>,
    ) -> Self {
        let mut semantics = resolve_invocation_semantics(class_spec, Some(method), form, false);
        semantics.options = invocation_options(class_spec, Some(method), form, availability);
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
        }
    }

    /// Resolve the complete mutable-world footprint for this invocation.
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
    pub fn effect_footprint(&self) -> EffectFootprint {
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
                match self.semantics.frame_effect.map(|effect| {
                    if successful {
                        effect.successful_layout(arguments).layout
                    } else {
                        effect.resolve_arguments(arguments)
                    }
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

    fn argument_roles_on_edge(&self, successful: bool) -> (Vec<(u8, ArgRole)>, bool) {
        let (mut arg_roles, mut arg_roles_complete) = if let Some(effect) =
            self.semantics.frame_effect
            && matches!(
                effect.layout,
                crate::FrameArgLayout::ScriptInSelectedFrame | crate::FrameArgLayout::AliasPairs
            ) {
            let layout = if successful {
                effect.successful_layout(self.words.arguments()).layout
            } else {
                effect.resolve_arguments(self.words.arguments())
            };
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
        self.extend_repeated_roles(&mut arg_roles, &mut arg_roles_complete, successful);
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
                    arg_roles_complete = false;
                }
            }
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
    fn selected_option_value_roles_reach_owned_facts_without_classifying_result_data() {
        let registry = CommandRegistry::build_default();
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
        let generic_effects = generic.effect_footprint();
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
            .effect_footprint();
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
        let command_effects = command.effect_footprint();
        assert_eq!(command_effects.accesses().len(), 1);
        assert_eq!(
            command_effects.accesses()[0].domain,
            WorldStateDomain::PackageState,
            "the command descriptor applies without a narrower match"
        );

        let subcommand = registry
            .resolve_invocation("world-effect-fixture", &["sub"], None)
            .expect("fixture subcommand resolves");
        let subcommand_effects = subcommand.effect_footprint();
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
        let form_effects = form.effect_footprint();
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
        let refined_effects = refined.effect_footprint();
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
        let effects = invocation.effect_footprint();
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
                    if alias.local == TransitionSubject::Literal("linked".to_owned())
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
                name: TransitionSubject::Literal("::precise".to_owned()),
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
                namespace: NamespaceTransitionTarget::Named(TransitionSubject::Literal(name)),
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
                interpreter: Some(TransitionSubject::Literal(path)),
                name: TransitionSubject::Literal(name),
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
                        if alias.local == TransitionSubject::Literal("local".to_owned())
                            && alias.target == VariableAliasTarget::CallerSelectedFrame {
                                frame: CallerFrameSelection::Explicit(
                                    TransitionSubject::Literal("1".to_owned())
                                ),
                                variable: TransitionSubject::Literal("other".to_owned()),
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
                        if alias.local == TransitionSubject::Literal("local".to_owned())
                            && alias.target == VariableAliasTarget::Namespace {
                                namespace: TransitionSubject::Literal("::scope".to_owned()),
                                variable: TransitionSubject::Literal("other".to_owned()),
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
        let footprint = invocation.effect_footprint();

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
