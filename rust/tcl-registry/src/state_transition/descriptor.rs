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

//! What a command declares about its transitions and how a descriptor resolves against one invocation.

use super::{
    StateTransitionCommit, StateTransitionDomain, StateTransitionOperandLayout, StateTransitions,
};
use crate::invocation_words::InvocationArguments;
use crate::world_effect::{TransitionEffectCoverage, TransitionEffectCoverages};

/// Whether a registry invocation has a closed state-transition declaration.
///
/// An unstamped invocation is a wildcard over every [`StateTransitionDomain`]
/// and is not equivalent to an empty fact list. Consumers must widen their
/// tracked command, cell, namespace, interpreter, trace, and object state for
/// [`Self::UnknownInvocation`]. Only [`Self::Declared`] — including an
/// explicit [`StateTransitionDescriptor::EMPTY`] — permits a consumer to use
/// the contained fact list as a closed registry statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateTransitionKnowledge {
    /// No descriptor was declared; the generic Tcl invocation can affect any
    /// tracked identity domain.
    UnknownInvocation,
    /// A registry descriptor supplied the complete transition fact list.
    Declared(StateTransitions),
}

impl StateTransitionKnowledge {
    /// Return whether the registry declared a closed transition description.
    #[must_use]
    pub const fn is_declared(&self) -> bool {
        matches!(self, Self::Declared(_))
    }

    /// Return the closed transition facts, when the registry declared them.
    #[must_use]
    pub fn declared(&self) -> Option<&StateTransitions> {
        match self {
            Self::UnknownInvocation => None,
            Self::Declared(transitions) => Some(transitions),
        }
    }
}

/// Explicit source-purpose dispatch registered by the descriptor producer.
/// The resolver's address never classifies its semantics or Native identity.
#[derive(Debug, Clone, Copy)]
pub enum LogicalSourceStateTransitionResolver {
    /// Conditional frame-alias grammar under positively retained Logical input.
    FrameAliases(StateTransitionResolver),
    /// Actual derived alias-pair producer, retaining the selected layout policy.
    /// Only this typed registration may supply conditional no-level roles.
    FrameAliasPairs {
        /// Actual producer-selected optional level grammar.
        level_word: crate::frame_effect::FrameLevelWord,
        /// Existing source-role resolver from that same producer.
        resolver: StateTransitionResolver,
    },
    /// Conditional completion storage through the retained source argv grammar.
    /// This supplies no Native completion, entered frame or successful edge.
    CompletionState(StateTransitionResolver),
}

/// Argument-sensitive resolver for a state transition descriptor.
///
/// The resolver sees only [`InvocationArguments`]' safe accessors. It can
/// retain literal subjects precisely with [`TransitionSubject::from_argument`]
/// and receives typed unknown subjects for dynamic, expanded, or opaque words.
pub type StateTransitionResolver = for<'w> fn(InvocationArguments<'w>) -> StateTransitions;

/// A state-domain widening declaration for identity-bearing argument
/// positions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StateTransitionWideningRule {
    /// Invocation positions whose dynamic values widen `domains`.
    pub operands: StateTransitionOperandLayout,
    /// State domains made unknown by those operands.
    pub domains: &'static [StateTransitionDomain],
}

/// Whether expansion or opacity can alter the positional grammar a resolver
/// uses to interpret operands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StateTransitionArgumentShape {
    /// Each argument can be interpreted independently; known arguments stay
    /// precise around an expanded sibling.
    Independent,
    /// Pairing or fixed positions matter. Expanded or opaque arguments
    /// suppress precise resolver facts and widen every declared domain.
    Positional,
}

/// How a narrower transition descriptor combines with a parent descriptor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StateTransitionComposition {
    /// Keep parent transitions and append the narrower descriptor's facts.
    Extend,
    /// Replace parent transition facts with the narrower descriptor's facts.
    Replace,
}

/// Registry data for command, subcommand, or form state transitions.
///
/// `dynamic_widening` is enforced centrally only for declared
/// identity-bearing positions, even if a resolver accidentally omits a
/// matching unknown branch. This preserves known `proc` names when unrelated
/// parameter/body words are dynamic, while still widening positional layouts
/// around expansion and opacity.
#[derive(Debug, Clone, Copy)]
pub struct StateTransitionDescriptor {
    /// How this descriptor composes with a less-specific descriptor.
    pub composition: StateTransitionComposition,
    /// Optional resolver for precise transition operands.
    pub resolver: Option<StateTransitionResolver>,
    /// Optional transition projection valid only on successful completion.
    /// Ordinary/error facts retain `resolver`; consumers must select the edge.
    pub success_resolver: Option<StateTransitionResolver>,
    /// Explicit readonly Logical alias grammar, separate from ordinary and
    /// successful Native transition resolvers.
    pub logical_source_resolver: Option<LogicalSourceStateTransitionResolver>,
    /// Whether expansion changes the resolver's positional grammar.
    pub argument_shape: StateTransitionArgumentShape,
    /// Domains widened for dynamic identity-bearing operands.
    pub dynamic_widening: &'static [StateTransitionWideningRule],
    /// Legacy or explicit world-effect writes for which the resolved
    /// completion-edge transitions are the authoritative representation.
    ///
    /// The contract is source-qualified, so declaring a command-binding
    /// transition cannot erase an independent variable or host write.
    pub effect_coverage: &'static [TransitionEffectCoverage],
    /// Completion edge on which this descriptor's transitions may commit.
    pub commit: StateTransitionCommit,
}

impl StateTransitionDescriptor {
    /// Explicitly declares that this invocation shape establishes no tracked
    /// state transitions.
    pub const EMPTY: Self = Self {
        composition: StateTransitionComposition::Extend,
        resolver: None,
        logical_source_resolver: None,
        success_resolver: None,
        argument_shape: StateTransitionArgumentShape::Independent,
        dynamic_widening: &[],
        effect_coverage: TransitionEffectCoverage::NONE,
        commit: StateTransitionCommit::OnOkOnly,
    };

    /// Resolve one descriptor against structured invocation arguments.
    #[must_use]
    pub fn resolve(self, arguments: InvocationArguments<'_>) -> StateTransitions {
        self.resolve_on_edge(arguments, false)
    }

    /// Resolve facts on the normal completion edge, preserving ordinary/error
    /// facts in [`Self::resolve`].
    #[must_use]
    pub fn resolve_after_success(self, arguments: InvocationArguments<'_>) -> StateTransitions {
        self.resolve_on_edge(arguments, true)
    }

    fn resolve_on_edge(
        self,
        arguments: InvocationArguments<'_>,
        successful: bool,
    ) -> StateTransitions {
        let positional_shape = self.argument_shape == StateTransitionArgumentShape::Positional;
        let mut transitions = if positional_shape && !arguments.has_exact_argv_len() {
            StateTransitions::default()
        } else {
            let resolver = if successful {
                self.success_resolver.or(self.resolver)
            } else {
                self.resolver
            };
            resolver.map_or_else(StateTransitions::default, |resolver| resolver(arguments))
        };
        transitions.widen_dynamic_arguments(arguments, self.dynamic_widening, positional_shape);
        transitions.set_commit(self.commit);
        transitions
    }
}

/// The command/subcommand/form transition descriptor chain selected for an
/// invocation. It stays borrowed and allocation-free until a common consumer
/// requests [`Self::resolve`].
#[derive(Debug, Clone, Copy, Default)]
pub struct ResolvedStateTransitions {
    /// Parent command transition descriptor.
    pub command: Option<StateTransitionDescriptor>,
    /// Resolved subcommand transition descriptor.
    pub subcommand: Option<StateTransitionDescriptor>,
    /// Matched form transition descriptor.
    pub form: Option<StateTransitionDescriptor>,
}

impl ResolvedStateTransitions {
    /// Whether this invocation has any registry-owned transition descriptor.
    #[must_use]
    pub const fn is_declared(self) -> bool {
        self.command.is_some() || self.subcommand.is_some() || self.form.is_some()
    }

    /// Resolve command, subcommand, then form facts, applying the narrower
    /// descriptor last so a form can explicitly replace a broader shape.
    #[must_use]
    pub fn resolve(self, arguments: InvocationArguments<'_>) -> StateTransitions {
        self.resolve_with_effect_coverage(arguments).0
    }

    /// Resolve transitions and the corresponding write-coverage contract as
    /// one registry operation. Coverage is selected only when its descriptor
    /// produced at least one transition (including a dynamic widening), so a
    /// no-op shape never suppresses an ordinary effect by itself.
    #[must_use]
    pub fn resolve_with_effect_coverage(
        self,
        arguments: InvocationArguments<'_>,
    ) -> (StateTransitions, TransitionEffectCoverages) {
        self.resolve_coverage_on_edge(arguments, false)
    }

    /// Readonly facts through explicitly registered source frame-alias
    /// resolvers. Custom resolvers, descriptor
    /// replacement, operands and commit annotations retain their own metadata.
    pub(crate) fn resolve_logical_source_frame_aliases(
        self,
        arguments: InvocationArguments<'_>,
    ) -> StateTransitions {
        let mut transitions = StateTransitions::default();
        for descriptor in [self.command, self.subcommand, self.form]
            .into_iter()
            .flatten()
        {
            let mut resolved = match descriptor.logical_source_resolver {
                Some(
                    LogicalSourceStateTransitionResolver::FrameAliases(resolver)
                    | LogicalSourceStateTransitionResolver::FrameAliasPairs { resolver, .. },
                ) => resolver(arguments),
                Some(LogicalSourceStateTransitionResolver::CompletionState(_)) | None => {
                    descriptor.resolve(arguments)
                }
            };
            resolved.widen_dynamic_arguments(
                arguments,
                descriptor.dynamic_widening,
                descriptor.argument_shape == StateTransitionArgumentShape::Positional,
            );
            resolved.set_commit(descriptor.commit);
            if descriptor.composition == StateTransitionComposition::Replace {
                transitions = resolved;
            } else {
                transitions.extend(resolved);
            }
        }
        transitions
    }

    pub(crate) fn resolve_authored_source_completion(
        self,
        arguments: InvocationArguments<'_>,
    ) -> StateTransitions {
        let mut transitions = StateTransitions::default();
        for descriptor in [self.command, self.subcommand, self.form]
            .into_iter()
            .flatten()
        {
            let mut resolved = match descriptor.logical_source_resolver {
                Some(LogicalSourceStateTransitionResolver::CompletionState(resolver)) => {
                    resolver(arguments)
                }
                Some(
                    LogicalSourceStateTransitionResolver::FrameAliases(_)
                    | LogicalSourceStateTransitionResolver::FrameAliasPairs { .. },
                )
                | None => descriptor.resolve(arguments),
            };
            resolved.widen_dynamic_arguments(
                arguments,
                descriptor.dynamic_widening,
                descriptor.argument_shape == StateTransitionArgumentShape::Positional,
            );
            resolved.set_commit(descriptor.commit);
            if descriptor.composition == StateTransitionComposition::Replace {
                transitions = resolved;
            } else {
                transitions.extend(resolved);
            }
        }
        transitions
    }

    /// Resolve normal-edge transitions and their matching write coverage.
    #[must_use]
    pub fn resolve_after_success_with_effect_coverage(
        self,
        arguments: InvocationArguments<'_>,
    ) -> (StateTransitions, TransitionEffectCoverages) {
        self.resolve_coverage_on_edge(arguments, true)
    }

    fn resolve_coverage_on_edge(
        self,
        arguments: InvocationArguments<'_>,
        successful: bool,
    ) -> (StateTransitions, TransitionEffectCoverages) {
        let mut transitions = StateTransitions::default();
        let mut coverage = TransitionEffectCoverages::default();
        for descriptor in [self.command, self.subcommand, self.form]
            .into_iter()
            .flatten()
        {
            let resolved = descriptor.resolve_on_edge(arguments, successful);
            let produced_transition = !resolved.facts().is_empty();
            if descriptor.composition == StateTransitionComposition::Replace {
                transitions = resolved;
                coverage.clear();
            } else {
                transitions.extend(resolved);
            }
            if produced_transition {
                coverage.extend(descriptor.effect_coverage);
            }
        }
        (transitions, coverage)
    }
}
