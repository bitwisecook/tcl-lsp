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
use crate::frame_effect::{FrameEffectSpec, FrameLevel, FrameLevelWord};
use crate::hooks::{CodegenHookId, InlineCodegenHookId, LoweringHookId};
use crate::hover::OptionSpec;
use crate::intrinsic::IntrinsicId;
use crate::invocation_words::{
    CommandPrefixArguments, InvocationArgument, InvocationWord, InvocationWordKind, InvocationWords,
};
use crate::literal_validation::{LiteralArgumentValidation, LiteralArgumentValidator};
use crate::option_effect::{OptionEffectScope, OptionEffects};
use crate::representation::RepresentationEffect;
use crate::result_stability::ResultStability;
use crate::semantic_operation::SemanticOperationId;
use crate::side_effects::SideEffect;
use crate::spec::{ArgRoleResolver, CaseInvocation, CommandSpec, InlineCaseClause, SubCommand};
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
    let (arg_roles, arg_role_resolver, clause_grammar) = match form {
        Some(form) => (form.arg_roles, None, None),
        None => match sub {
            Some(sub) => (sub.arg_roles, sub.arg_role_resolver, sub.clause_grammar),
            None => (spec.arg_roles, spec.arg_role_resolver, spec.clause_grammar),
        },
    };
    let inherited_traits = if inherit_command {
        spec.traits
    } else {
        Traits::empty()
    } | sub.map_or_else(Traits::empty, |sub| {
        sub.traits
            | if sub.pure {
                Traits::PURE
            } else {
                Traits::empty()
            }
    });
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
        representation_effect: form
            .and_then(|form| form.representation_effect)
            .or(sub.and_then(|sub| sub.representation_effect))
            .or(inherit_command
                .then_some(spec.representation_effect)
                .flatten())
            .unwrap_or_default(),
        traits: form
            .and_then(|form| form.traits)
            .unwrap_or(inherited_traits),
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
        clause_grammar,
        repeated_args: sub.map_or(spec.repeated_args, |sub| sub.repeated_args),
        options: InvocationOptions {
            base: sub.map_or(spec.options, |sub| sub.options),
            form: form.map_or(&[], |form| form.options),
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
        return_type: sub.map_or(spec.return_type, |sub| sub.return_type),
        safe_on_uninit: sub
            .and_then(|sub| sub.safe_on_uninit)
            .or(inherit_command.then_some(spec.safe_on_uninit).flatten()),
        var_write_typing: sub.map_or(spec.var_write_typing, |sub| sub.var_write_typing),
        return_elements: sub.map_or(spec.return_elements, |sub| sub.return_elements),
        var_elements_effect: sub.map_or(spec.var_elements_effect, |sub| sub.var_elements_effect),
        frame_effect: inherit_command.then_some(spec.frame_effect).flatten(),
        body_kind: sub.map_or(spec.body_kind, |sub| sub.body_kind),
        body_interpreter: sub.map_or(spec.body_interpreter, |sub| sub.body_interpreter),
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

/// Option descriptors applicable to a resolved invocation.
///
/// Command and subcommand options remain separate from form-local options so
/// the resolver remains allocation-free.  Consumers must consult both slices;
/// this mirrors the registry keyword-table construction.
#[derive(Debug, Clone, Copy)]
pub struct InvocationOptions<'r> {
    /// Options declared by the command or resolved subcommand.
    pub base: &'r [OptionSpec],
    /// Options declared only by the matched form.
    pub form: &'r [OptionSpec],
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
    /// Registry-selected static semantic operation identity.
    ///
    /// This is not a live command identity; runtime command binding and trace
    /// state are established by later common analyses.
    pub operation: SemanticOperationId,
    /// Effective target-neutral completion contract.
    ///
    /// The resolver selects a matching form first, then a resolved subcommand,
    /// then the command descriptor, falling back to the conservative generic
    /// invoke contract only when the registry supplied none of them.
    pub completion: CompletionDescriptor,
    /// Registry-declared dependency contract for the invocation's result.
    pub result_stability: ResultStability,
    /// Effective Tcl value-representation effect.
    pub representation_effect: RepresentationEffect,
    /// Additive command and subcommand behaviour traits.
    pub traits: Traits,
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
    /// The effective clause grammar — the command's, or the resolved
    /// subcommand's — read by [`ResolvedInvocation::clause_plan`]. A matched
    /// form has only static roles and therefore supplies `None`.
    pub clause_grammar: Option<&'r crate::clause_grammar::ClauseGrammarSpec>,
    /// The effective repeated-argument layouts, which a clause grammar's group
    /// rows cite.
    pub repeated_args: &'r [crate::repeated::RepeatedArgLayout],
    /// Effective command/subcommand/form option descriptors.
    pub options: InvocationOptions<'r>,
    /// Where the options' effects come from — the families, reservation,
    /// prefix policy and inherited release gate of the selected option table
    /// (read by [`ResolvedInvocation::option_effects`]).
    pub option_scope: OptionEffectScope<'r>,
    /// Result Tcl internal-representation type, when declared.
    pub return_type: Option<TclType>,
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

/// An owned, target-neutral projection of a resolved registry invocation.
///
/// This is the hand-off shape for executable IR and shared optimisation
/// passes.  It contains facts selected by the registry, not a claim that the
/// command's runtime binding, namespace lookup, aliases, traces, or `unknown`
/// handling have been proven stable.  It deliberately excludes every backend
/// and lowering hook.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvocationFacts {
    /// Canonical registry command identity used to select these facts.
    pub canonical_command: String,
    /// The complete owned subcommand-resolution outcome.
    pub subcommand: OwnedSubcommandResolution,
    /// Registry form identity, when a form was selected from a determinate
    /// argv shape.
    pub form: Option<String>,
    /// Target-neutral semantic operation selected by the registry.
    pub operation: SemanticOperationId,
    /// Effective Tcl completion descriptor.
    pub completion: CompletionDescriptor,
    /// Registry-declared dependency contract for the invocation's result.
    pub result_stability: ResultStability,
    /// Effective Tcl value-representation effect.
    pub representation_effect: RepresentationEffect,
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
    /// Whether the selected invocation form mutates command-specific state.
    pub mutator: bool,
    /// Effective arity after command, subcommand, and form selection.
    pub arity: Arity,
    /// Number of leading post-head words before static argument roles start.
    pub argument_offset: usize,
    /// Effective argument-role declarations for this invocation.
    ///
    /// A registry resolver is evaluated here when every argument is literal.
    /// Otherwise this retains the descriptor's conservative static roles and
    /// [`Self::arg_roles_complete`] is `false`.
    pub arg_roles: Vec<(u8, ArgRole)>,
    /// Whether [`Self::arg_roles`] is the complete role assignment.
    ///
    /// `false` means registry metadata has an argument-role resolver whose
    /// result depends on literal invocation arguments. Consumers must retain
    /// the dynamic role obligation rather than treating this static slice as a
    /// complete answer.
    pub arg_roles_complete: bool,
    /// Declared result internal-representation type, when available.
    pub return_type: Option<TclType>,
    /// How written variables receive types from this invocation.
    pub var_write_typing: VarWriteTyping,
    /// Result-to-container-element relationship, when declared.
    pub return_elements: Option<ReturnElements>,
    /// In-place container-element evolution, when declared.
    pub var_elements_effect: Option<VarElementsEffect>,
    /// Whether body words run in the caller frame or a structural scope.
    pub body_kind: BodyKind,
    /// Which interpreter owns evaluated body arguments.
    pub body_interpreter: BodyInterpreter,
    /// Frame-crossing argument grammar, when declared.
    pub frame_effect: Option<FrameEffectSpec>,
}

impl InvocationFacts {
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
        let effective_count = argument_count.checked_sub(self.argument_offset)?;
        let effective_count = u16::try_from(effective_count).ok()?;
        if !self.arity.accepts(effective_count) || !self.arg_roles_complete {
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

impl<'r, 'w> ResolvedInvocation<'r, 'w> {
    pub(crate) fn new(
        words: InvocationWords<'w>,
        spec: &'r CommandSpec,
        sub: Option<&'r SubCommand>,
        form: Option<&'r CommandForm>,
        subcommand: SubcommandResolution<'w>,
        dialect: Option<SurfaceQuery<'w>>,
    ) -> Self {
        let semantics = resolve_invocation_semantics(spec, sub, form, true);
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
        dialect: Option<SurfaceQuery<'w>>,
    ) -> Self {
        let semantics = resolve_invocation_semantics(class_spec, Some(method), form, false);
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
        let dialect = self.dialect;
        let grammar = self.semantics.clause_grammar?;
        if !grammar.available(dialect) {
            return None;
        }
        let arguments = self.words.arguments();
        let len = arguments.exact_argv_len()?;
        let offset = self.semantics.argument_offset.min(len);
        let values: Vec<&str> = (offset..len)
            .map(|index| arguments.literal_at(index).unwrap_or(""))
            .collect();
        let dynamic: Vec<bool> = (offset..len)
            .map(|index| arguments.literal_at(index).is_none())
            .collect();
        Some(
            grammar
                .walk_words_or_abstain(&values, &dynamic, self.semantics.repeated_args, dialect)
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
        match self.dialect?.core {
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
        if !self.layout_is_proven() {
            return None;
        }
        let spellings = self.placeholder_spellings()?;
        let SelectedDescriptors { spec, sub, .. } = self.selected;
        let mut roles = crate::registry::arg_roles_in(
            spec,
            sub,
            &spellings,
            |_| true,
            self.dialect,
            // The registry's role answer gates a case-list command's body
            // roles on the placeholder reading; so does this.
            || {
                spec.case_list.is_some_and(|case| {
                    case.invocation(&spellings, &self.spec_options(), self.dialect)
                        .is_some()
                })
            },
            || {
                self.pattern_args()
                    .into_iter()
                    .map(|pattern| usize::from(pattern.index))
                    .collect()
            },
        );
        roles.extend(
            self.command_prefixes_over(&spellings)
                .into_iter()
                .map(|(index, _)| (index, ArgRole::CommandPrefix)),
        );
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
                crate::registry::arg_roles_in(
                    spec,
                    sub,
                    &spellings,
                    |role| role == ArgRole::Pattern,
                    self.dialect,
                    || true,
                    Vec::new,
                )
                .into_iter()
                .map(|(index, _)| index)
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
        let invocation = case.invocation(&spellings, &options, self.dialect)?;
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
        let arguments = self.words.arguments();
        let len = arguments.exact_argv_len()?;
        let version = self.tcl_version();
        let taken = match frame.level_word {
            FrameLevelWord::None | FrameLevelWord::ArityParity => {
                frame.level_word_len_for_argument_count(len)?
            }
            FrameLevelWord::LeadingProbe => match arguments.argv_at(0) {
                InvocationArgument::Word(InvocationWord::Literal(word)) => {
                    frame.level_word_len_for_version(&[word, ""][..len.min(2)], version)
                }
                // A computed word separates from the script only when a
                // script word follows it.
                InvocationArgument::Word(_) => usize::from(len >= 2),
                InvocationArgument::Missing | InvocationArgument::Indeterminate => 0,
            },
        };
        let level = if taken == 0 {
            FrameLevel::DEFAULT
        } else {
            arguments
                .literal_at(0)
                .and_then(|word| FrameLevel::parse_for(word, version))
                .unwrap_or(FrameLevel::Dynamic)
        };
        Some((level, (taken..len).map(OperandId).collect()))
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
        let SelectedDescriptors {
            spec,
            sub,
            instance,
        } = self.selected;
        if instance || !spec.subcommands.is_empty() {
            return sub.and_then(|sub| sub.return_type);
        }
        match spec.return_type_hook {
            Some(hook) => crate::return_type::resolve(hook, spec, &self.hook_spellings()?),
            None => spec.return_type,
        }
    }

    /// The words as the return-type hooks read them — source spellings, in
    /// which a substituted word shows its `$` — or `None` under an
    /// expansion. A computed word is spelt `$`, which a hook reads as a
    /// value it cannot see, never as a switch or a literal operand.
    fn hook_spellings(&self) -> Option<Vec<&'w str>> {
        let arguments = self.words.arguments();
        (0..arguments.len())
            .map(|index| match arguments.get(index)? {
                InvocationWord::Literal(word) => Some(word),
                InvocationWord::Dynamic | InvocationWord::DynamicNonOption => Some("$"),
                InvocationWord::Expanded | InvocationWord::Opaque => None,
            })
            .collect()
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

    /// The effective flat role table over the literal words, and whether it is
    /// complete.
    ///
    /// A dynamic resolver — or a clause grammar standing where a retired
    /// resolver stood, on a descriptor with no static table — needs every
    /// word literal; otherwise the static table is the answer and is marked
    /// incomplete. The grammar's roles are appended after the resolver's or
    /// the static table's, never reordering them.
    fn role_table(&self) -> (Vec<(u8, ArgRole)>, bool) {
        let semantics = &self.semantics;
        let grammar_is_role_source =
            semantics.clause_grammar.is_some() && semantics.arg_roles.is_empty();
        if semantics.arg_role_resolver.is_none() && !grammar_is_role_source {
            return (semantics.arg_roles.to_vec(), true);
        }
        let Some(arguments) = self
            .words
            .arguments()
            .literal_values()
            .and_then(|arguments| {
                arguments
                    .get(semantics.argument_offset..)
                    .map(<[_]>::to_vec)
            })
        else {
            return (semantics.arg_roles.to_vec(), false);
        };
        let mut roles = semantics.arg_role_resolver.map_or_else(
            || semantics.arg_roles.to_vec(),
            |resolver| resolver(&arguments),
        );
        if let Some(grammar) = semantics.clause_grammar {
            let plan = grammar.walk(&arguments, semantics.repeated_args);
            for (index, role) in plan.roles {
                if let Ok(index) = u8::try_from(index)
                    && !roles.contains(&(index, role))
                {
                    roles.push((index, role));
                }
            }
        }
        (roles, true)
    }

    /// Materialise the target-neutral facts for an owned consumer such as an
    /// executable IR node.
    ///
    /// Borrowed resolution itself stays allocation-free.  This method is the
    /// explicit ownership boundary: it copies registry identities and
    /// resolves any lazy world-effect descriptors.
    #[must_use]
    pub fn facts(&self) -> InvocationFacts {
        let (transitions, transition_effect_coverage) = self
            .semantics
            .state_transitions
            .resolve_with_effect_coverage(self.words.arguments());
        let (arg_roles, arg_roles_complete) = self.role_table();
        InvocationFacts {
            canonical_command: self.canonical_command.to_owned(),
            subcommand: self.subcommand.into_owned(),
            form: self.form.map(|form| form.name.to_owned()),
            operation: self.semantics.operation,
            completion: self.semantics.completion,
            result_stability: self.semantics.result_stability,
            representation_effect: self.semantics.representation_effect,
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
            mutator: self.semantics.mutator,
            arity: self.semantics.arity,
            argument_offset: self.semantics.argument_offset,
            arg_roles,
            arg_roles_complete,
            return_type: self.semantics.return_type,
            var_write_typing: self.semantics.var_write_typing,
            return_elements: self.semantics.return_elements,
            var_elements_effect: self.semantics.var_elements_effect,
            body_kind: self.semantics.body_kind,
            body_interpreter: self.semantics.body_interpreter,
            frame_effect: self.semantics.frame_effect,
        }
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
    use tcl_dialect::model::SpecSurface;
    use tcl_dialect::model::{Family, SurfaceQuery};

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
