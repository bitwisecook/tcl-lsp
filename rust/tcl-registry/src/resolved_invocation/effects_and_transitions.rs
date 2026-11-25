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

//! Side effects, state transitions and variable aliases of one invocation, as declared and as the authored source narrows them.

use super::ResolvedInvocation;
use crate::arg_role::ArgRole;
use crate::hooks::LoweringHookId;
use crate::semantic_operation::SemanticOperationId;
use crate::side_effects::SideEffect;
use crate::state_transition::StateTransitions;
use crate::traits::Traits;
use crate::world_effect::{EffectFootprint, TransitionEffectCoverages};

impl<'r> ResolvedInvocation<'r, '_> {
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

    pub(super) fn effect_footprint_with_transition_coverage(
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
        // Selected transitions own command-table mutation and its coverage.
        footprint.extend(self.legacy_effect_footprint(command_table_mutation, coverage));
        footprint
    }

    /// Legacy hints declared by this actual selected descriptor and form.
    /// An empty slice remains unknown, never a search for an older available
    /// declaration. Availability, normal entry and full effects are separate.
    #[must_use]
    pub fn selected_side_effect_hints(&self) -> &'r [SideEffect] {
        self.semantics.side_effects.lower_bound()
    }

    fn legacy_effect_footprint(
        &self,
        command_table_mutation: bool,
        coverage: &TransitionEffectCoverages,
    ) -> EffectFootprint {
        let mut footprint = EffectFootprint::from_legacy_with_transition_coverage(
            command_table_mutation,
            self.semantics.frame_effect,
            &[],
            coverage,
        );
        for effect in self.selected_side_effect_hints() {
            if effect.surface.is_none_or(|surface| {
                tcl_dialect::model::surface_admits(surface, self.dialect.as_ref())
            }) {
                footprint.extend(EffectFootprint::from_legacy_with_transition_coverage(
                    false,
                    None,
                    std::slice::from_ref(effect),
                    coverage,
                ));
            }
        }
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

    /// Conditional state facts under the original Logical source grammar.
    /// Registered source frame-alias resolvers use authored layout consensus;
    /// actual input remains caller-owned, with no frame or completion grant.
    #[must_use]
    pub fn authored_logical_source_state_transitions(&self) -> Option<StateTransitions> {
        let selected = [
            self.semantics.state_transitions.command,
            self.semantics.state_transitions.subcommand,
            self.semantics.state_transitions.form,
        ]
        .into_iter()
        .flatten()
        .fold(false, |selected, descriptor| {
            let own = matches!(
                descriptor.logical_source_resolver,
                Some(
                    crate::state_transition::LogicalSourceStateTransitionResolver::FrameAliases(_)
                    | crate::state_transition::LogicalSourceStateTransitionResolver::FrameAliasPairs { .. }
                )
            );
            if descriptor.composition == crate::StateTransitionComposition::Replace {
                own
            } else {
                selected || own
            }
        });
        if selected {
            use crate::frame_effect::FrameArgumentResolution;
            match self.authored_logical_source_frame_argument_layout()? {
                FrameArgumentResolution::Unknown
                | FrameArgumentResolution::Valid {
                    level: crate::FrameLevel::Dynamic,
                    ..
                } => return None,
                FrameArgumentResolution::Valid { .. } | FrameArgumentResolution::Invalid => {}
            }
        }
        Some(
            self.semantics
                .state_transitions
                .resolve_logical_source_frame_aliases(self.words.arguments()),
        )
    }

    /// Finite conditional no-level roles from an explicitly registered derived
    /// alias-pair producer. The ordinary/generic source layout remains Unknown.
    /// No actual default caller, selected local, completion or absence follows.
    #[must_use]
    pub fn conditional_no_level_source_alias_layout(
        &self,
    ) -> Option<ConditionalNoLevelAliasLayout> {
        use crate::state_transition::LogicalSourceStateTransitionResolver as Resolver;
        let mut selected = None;
        for descriptor in [
            self.semantics.state_transitions.command,
            self.semantics.state_transitions.subcommand,
            self.semantics.state_transitions.form,
        ]
        .into_iter()
        .flatten()
        {
            if descriptor.composition == crate::StateTransitionComposition::Replace {
                selected = None;
            }
            if let Some(Resolver::FrameAliasPairs { level_word, .. }) =
                descriptor.logical_source_resolver
            {
                if selected.is_some_and(|prior| prior != level_word) {
                    return None;
                }
                selected = Some(level_word);
            }
        }
        let level_word = selected?;
        let frame = self.semantics.frame_effect?;
        if frame.level_word != level_word
            || self.facts().arity_accepts_frozen_arguments() != Some(true)
        {
            return None;
        }
        let offset = self.semantics.argument_offset;
        let arguments = self.words.arguments();
        frame.conditional_no_level_source_layout(arguments.slice_from(offset))?;
        let mut pairs = Vec::new();
        for target in (offset..arguments.exact_argv_len()?).step_by(2) {
            let variable = crate::TransitionSubject::from_argument(arguments, target)?;
            let local = crate::TransitionSubject::from_argument(arguments, target + 1)?;
            // A computed local cannot issue an independently written local name.
            if local.literal().is_some() {
                pairs.push((variable, local));
            }
        }
        if pairs.is_empty() {
            return None;
        }
        Some(ConditionalNoLevelAliasLayout {
            pairs,
            residual: self
                .semantics
                .state_transitions
                .resolve_logical_source_frame_aliases(arguments),
        })
    }

    /// Conditional authored effects and state facts from the same actual argv.
    /// The caller retains the source grammar independently of handler metadata.
    /// Registered source resolvers preserve uncertainty; this supplies no
    /// Native handler, completed effect or Normal proof.
    #[must_use]
    pub fn authored_source_effects_and_state_transitions(
        &self,
        source_dialect: crate::InvocationDialect,
    ) -> (EffectFootprint, StateTransitions) {
        let arguments = self.words.arguments().with_dialect(source_dialect);
        let transitions = self
            .semantics
            .state_transitions
            .resolve_authored_source_completion(arguments);
        let mut effects = if self.semantics.world_effects.is_declared() {
            self.semantics
                .world_effects
                .resolve_authored_source(arguments)
        } else {
            EffectFootprint::conservative_unknown_invocation()
        };
        effects.extend(self.legacy_effect_footprint(
            transitions.touches_command_bindings(),
            &TransitionEffectCoverages::default(),
        ));
        (effects, transitions)
    }

    /// Resolve source effects for a registry-declared delegated script while
    /// retaining the callback and every independently declared effect. The
    /// coverage accounts only for the registered legacy bridge; no body entry,
    /// Native handler, or completion follows from this source projection.
    #[must_use]
    pub fn authored_source_delegated_script_effects(
        &self,
        source_dialect: crate::InvocationDialect,
    ) -> Option<(EffectFootprint, StateTransitions)> {
        use crate::world_effect::SourceWorldEffectResolver;
        let arguments = self.words.arguments().with_dialect(source_dialect);
        if self
            .words
            .arguments()
            .dialect()
            .is_some_and(|actual| actual != source_dialect)
            || self.nul_free_literal_words(arguments).is_none()
            || !self
                .semantics
                .arity
                .accepts(self.argument_count_for_arity()?)
            || !matches!(
                crate::script_body_flow::script_body_flow_for_invocation(&self.facts(), arguments),
                crate::script_body_flow::ScriptBodyFlow::ConcatenatedScript { .. }
            )
        {
            return None;
        }
        let descriptor = sole_world_effect_descriptor(&self.semantics.world_effects)?;
        let SourceWorldEffectResolver::ResultTransitions { coverage, .. } =
            descriptor.source_resolver?
        else {
            return None;
        };
        let selected = coverage(arguments)?;
        let transitions = self
            .semantics
            .state_transitions
            .resolve_authored_source_completion(arguments);
        if transitions.facts().is_empty()
            || transitions != selected.transitions
            || self.semantics.side_effects.lower_bound() != selected.legacy_effects
        {
            return None;
        }
        let mut effects = self
            .semantics
            .world_effects
            .resolve_authored_source(arguments);
        effects.extend(
            self.legacy_effect_footprint(
                transitions.touches_command_bindings(),
                &selected.coverage,
            ),
        );
        Some((effects, transitions))
    }

    /// A conditional linear source continuation from an explicitly registered
    /// operation contract. No handler, observer, Normal or committed result is
    /// supplied. Full raw effects and result applicability remain on the card.
    #[must_use]
    pub fn authored_source_operation_continuation(
        &self,
        source_dialect: crate::InvocationDialect,
    ) -> Option<crate::source_operation::AuthoredSourceOperationContinuation> {
        use crate::world_effect::{EffectAccessMode, SourceWorldEffectResolver};
        let selectors = self.authored_source_option_selector_path()?;
        let arguments = self.words.arguments().with_dialect(source_dialect);
        let words = self.nul_free_literal_words(arguments)?;
        if self
            .words
            .arguments()
            .dialect()
            .is_some_and(|actual| actual != source_dialect)
            || self.semantics.frame_effect.is_some()
            || match self.semantics.body_kind {
                crate::BodyKind::Plain => false,
                crate::BodyKind::Structural => self
                    .semantics
                    .body_execution
                    .and_then(crate::body_execution::BodyExecutionSpec::deferred_contract)
                    .is_none(),
            }
            || self.semantics.body_interpreter != crate::BodyInterpreter::Current
            || !self
                .semantics
                .arity
                .accepts(self.argument_count_for_arity()?)
        {
            return None;
        }
        let facts = self.facts();
        if !matches!(
            crate::script_body_flow::script_body_flow_for_invocation(&facts, arguments),
            crate::script_body_flow::ScriptBodyFlow::None
                | crate::script_body_flow::ScriptBodyFlow::Deferred
        ) {
            return None;
        }
        let descriptor = sole_world_effect_descriptor(&self.semantics.world_effects)?;
        let SourceWorldEffectResolver::ResultTransitions { effects, coverage } =
            descriptor.source_resolver?
        else {
            return None;
        };
        let selected = coverage(arguments)?;
        let (raw, transitions) = self.authored_source_effects_and_state_transitions(source_dialect);
        if transitions != selected.transitions
            || transitions.facts().is_empty()
            || self.semantics.side_effects.lower_bound() != selected.legacy_effects
            || transitions.facts().iter().any(|fact| {
                fact.commit != crate::StateTransitionCommit::OnOkOnly
                    || matches!(fact.transition, crate::StateTransition::Widen(_))
            })
            || transitions.command_resolution_impact() != crate::CommandResolutionImpact::None
        {
            return None;
        }
        // Only this audited resolver's declared writes are covered. Additional
        // static descriptor accesses remain residual even in the same domain.
        let mut residual = crate::world_effect::WorldEffectDescriptor {
            static_footprint: crate::world_effect::StaticEffectFootprint::EMPTY,
            resolver: Some(effects),
            ..descriptor
        }
        .resolve_with_transition_coverage(arguments, &selected.coverage);
        residual.extend(
            crate::world_effect::WorldEffectDescriptor {
                resolver: None,
                ..descriptor
            }
            .resolve(arguments),
        );
        residual.extend(self.legacy_effect_footprint(false, &selected.coverage));
        if residual.requires_world_barrier()
            || residual
                .accesses()
                .iter()
                .any(|access| access.mode != EffectAccessMode::Read)
        {
            return None;
        }
        Some(
            crate::source_operation::AuthoredSourceOperationContinuation {
                descriptor: self.descriptor_key().label().to_owned(),
                selectors,
                form: self.form.map(|form| form.name),
                dispatch_dependencies: facts.dispatch_dependencies,
                words,
                dialect: source_dialect,
                effects: raw,
                residual,
                result_transitions: transitions,
                coverage: selected.coverage,
            },
        )
    }

    /// The NUL-free literal head and argument words of this invocation.
    fn nul_free_literal_words(
        &self,
        arguments: crate::InvocationArguments<'_>,
    ) -> Option<Vec<String>> {
        let count = arguments.exact_argv_len()?;
        let head = self.words.head_literal()?;
        if head.contains('\0') {
            return None;
        }
        let mut words = vec![head.to_owned()];
        for at in 0..count {
            let value = arguments.literal_at(at)?;
            if value.contains('\0') {
                return None;
            }
            words.push(value.to_owned());
        }
        Some(words)
    }

    /// Readonly aliases from the same Logical state-fact projection. Unknown
    /// source layouts abstain; successful linking remains an independent query.
    #[must_use]
    pub fn authored_logical_source_variable_aliases(
        &self,
    ) -> Option<Vec<crate::VariableCellAliasTransition>> {
        let transitions = self.authored_logical_source_state_transitions()?;
        Some(
            transitions
                .facts()
                .iter()
                .filter_map(|fact| match &fact.transition {
                    crate::StateTransition::VariableCellAlias(alias) => Some(alias.clone()),
                    _ => None,
                })
                .collect(),
        )
    }

    /// Alias destination applicability in an independently retained Logical
    /// Tcl source frame. The actual selected source descriptor must contain
    /// this destination. Unversioned Tcl uses unanimous authored release
    /// policies here; it does not acquire an execution policy, entered frame,
    /// completed link or ordinary absence proof.
    #[must_use]
    pub fn authored_logical_source_alias_destination_is_active_in_frame(
        &self,
        destination: crate::VariableAliasDestination,
        frame: crate::VariableAliasFrame,
    ) -> Option<bool> {
        let dialect = self.words.arguments().dialect()?;
        if dialect.family() != Some(tcl_dialect::model::Family::Tcl)
            || dialect.native_name_protocol().is_some()
            || dialect.core_point.is_some()
            || dialect.tcl_version.is_some()
            || dialect.variable_lookup_policy.is_some()
            || self.facts().arity_accepts_frozen_arguments() != Some(true)
            || !self
                .authored_logical_source_variable_aliases()?
                .iter()
                .any(|alias| alias.destination == destination)
        {
            return None;
        }
        let mut agreed = None;
        for version in tcl_dialect::TclVersion::ALL {
            let active = destination
                .is_active_in_frame(frame, crate::InvocationDialect::for_version(version))?;
            if agreed.is_some_and(|previous| previous != active) {
                return None;
            }
            agreed = Some(active);
        }
        agreed
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
}

/// The one world-effect descriptor left after replacing declarations apply.
fn sole_world_effect_descriptor(
    effects: &crate::world_effect::ResolvedWorldEffects,
) -> Option<crate::world_effect::WorldEffectDescriptor> {
    let mut descriptors = Vec::new();
    for descriptor in effects.stated_layers() {
        if descriptor.composition == crate::world_effect::WorldEffectComposition::Replace {
            descriptors.clear();
        }
        descriptors.push(descriptor);
    }
    let [descriptor] = descriptors.as_slice() else {
        return None;
    };
    Some(*descriptor)
}

/// Source-only positioned alias roles in the finite no-level branch of an
/// unresolved optional-level grammar. This is not a state transition or frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConditionalNoLevelAliasLayout {
    pairs: Vec<(crate::TransitionSubject, crate::TransitionSubject)>,
    residual: crate::StateTransitions,
}

impl ConditionalNoLevelAliasLayout {
    /// Actual target/local subjects; original Written-word correspondence is
    /// independently required by a source consumer.
    #[must_use]
    pub fn pairs(&self) -> &[(crate::TransitionSubject, crate::TransitionSubject)] {
        &self.pairs
    }
    /// Original unresolved state facts, including their variable/trace widening.
    #[must_use]
    pub fn residual(&self) -> &crate::StateTransitions {
        &self.residual
    }
    /// The alternate level interpretation may produce an argument error.
    #[must_use]
    pub const fn may_argument_error(&self) -> bool {
        true
    }
}
