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

//! Conditional source metadata retains callback uncertainty without effect grants.

use crate::intrinsic::IntrinsicId;
use crate::resolved_invocation::SourceExecutableValueCapability;
use crate::world_effect::EffectFootprint;
use crate::{
    BodyInterpreter, BodyKind, InvocationDialect, ResolvedInvocation, SemanticOperationId,
    StateTransitionKnowledge, StateTransitions,
};

/// Conditions on source assistance; none is an observed execution fact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionalSourceObserverObligation {
    /// The original operation must actually select this registered schema.
    SelectedOperationApplicability,
    /// Unknown message/option interpretation or I/O failure may stop continuation.
    NormalContinuation,
    /// Transforms/reflected channels may execute arbitrary Tcl or host callbacks.
    UnresolvedChannelCallbacks,
    /// Those callbacks may replace commands or alter subsequent lookup paths.
    LookupWorldAfterCallbacks,
}

/// Separate SOURCE metadata; retained effects remain conservatively unchanged.
/// This supplies no observer-free execution, actual lookup, Normal or edit proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConditionalSourceObserverEnvelope {
    dialect: InvocationDialect,
    operation: SemanticOperationId,
    ordinary_effects: EffectFootprint,
    ordinary_transitions: StateTransitionKnowledge,
    source_effects: EffectFootprint,
    source_transitions: StateTransitions,
}

impl ConditionalSourceObserverEnvelope {
    /// Full ordinary uncertainty, never replaced by an observer-free footprint.
    #[must_use]
    pub const fn ordinary_effects(&self) -> &EffectFootprint {
        &self.ordinary_effects
    }

    /// Full independently selected source uncertainty, including callbacks.
    #[must_use]
    pub const fn source_effects(&self) -> &EffectFootprint {
        &self.source_effects
    }

    /// All outstanding conditions remain attached to this metadata envelope.
    #[must_use]
    pub const fn obligations(&self) -> &'static [ConditionalSourceObserverObligation] {
        &[
            ConditionalSourceObserverObligation::SelectedOperationApplicability,
            ConditionalSourceObserverObligation::NormalContinuation,
            ConditionalSourceObserverObligation::UnresolvedChannelCallbacks,
            ConditionalSourceObserverObligation::LookupWorldAfterCallbacks,
        ]
    }
}

impl ResolvedInvocation<'_, '_> {
    /// Describe only the audited original single-message channel-write grammar.
    /// Unknown effects stay unknown; the caller must retain every callback and
    /// later-lookup condition. Metadata does not select the runtime handler.
    #[must_use]
    pub fn conditional_source_observer_envelope(
        &self,
        dialect: InvocationDialect,
    ) -> Option<ConditionalSourceObserverEnvelope> {
        let descriptor = self.authored_source_descriptors();
        if !std::ptr::eq(descriptor.command, self.selected.spec)
            || descriptor.subcommand.is_some()
            || self.selected.sub.is_some()
            || self.selected.instance
            || self.selected.form.is_some()
            || self.form.is_some()
            || self.semantics.operation != SemanticOperationId::Intrinsic(IntrinsicId::ChannelWrite)
            || descriptor.command.semantic_operation
                != Some(SemanticOperationId::Intrinsic(IntrinsicId::ChannelWrite))
            || !matches!(
                dialect.native_name_protocol(),
                Some(tcl_syntax::naming::NativeNameProtocol::C(_))
            )
            || self.words.arguments().dialect() != Some(dialect)
            || self.words.arguments().exact_argv_len() != Some(1)
            || self.semantics.argument_offset != 0
            || self.semantics.body_kind != BodyKind::Plain
            || self.semantics.body_interpreter != BodyInterpreter::Current
            || self.semantics.frame_effect.is_some()
            || self
                .semantics
                .traits
                .intersects(crate::traits::FRAME_REACH_TRAITS)
            || self.authored_source_executable_value_capability()
                != SourceExecutableValueCapability::Absent
        {
            return None;
        }
        let facts = self.facts();
        let (roles, complete) = self.authored_source_argument_roles();
        if facts.arity_accepts_frozen_arguments() != Some(true)
            || facts.arity_argument_count != Some(1)
            || !complete
            || !roles.is_empty()
        {
            return None;
        }
        let (source_effects, source_transitions) =
            self.authored_source_effects_and_state_transitions(dialect);
        Some(ConditionalSourceObserverEnvelope {
            dialect,
            operation: facts.operation,
            ordinary_effects: facts.effects,
            ordinary_transitions: facts.state_transitions,
            source_effects,
            source_transitions,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{InvocationWord, InvocationWords};

    fn envelope(
        head: &str,
        arguments: &[InvocationWord<'_>],
    ) -> Option<ConditionalSourceObserverEnvelope> {
        let dialect = InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
        let context = crate::model::ingress::static_context_for("tcl8.6");
        let resolution = crate::model::assembly::resolve_structured_invocation_in_resolved_context(
            context.commands(),
            Some(context.context()),
            InvocationWords::structured(InvocationWord::Literal(head), arguments)
                .with_dialect(dialect),
            tcl_dialect::model::InvocationRealm::RuleLoader,
        );
        resolution
            .resolved()?
            .conditional_source_observer_envelope(dialect)
    }

    #[test]
    fn implicit_channel_source_envelope_keeps_actual_unknown_effects_and_all_callback_conditions() {
        for version in tcl_dialect::TclVersion::ALL {
            let dialect = InvocationDialect::for_version(version);
            let context = crate::model::ingress::static_context_for(version.dialect_name());
            let arguments = [InvocationWord::Dynamic];
            let resolution =
                crate::model::assembly::resolve_structured_invocation_in_resolved_context(
                    context.commands(),
                    Some(context.context()),
                    InvocationWords::structured(InvocationWord::Literal("puts"), &arguments)
                        .with_dialect(dialect),
                    tcl_dialect::model::InvocationRealm::RuleLoader,
                );
            let selected = resolution.resolved().unwrap();
            let before = selected.facts();
            let envelope = selected
                .conditional_source_observer_envelope(dialect)
                .unwrap();
            assert!(before.effects.callback().kinds.is_unknown());
            assert!(before.effects.requires_world_barrier());
            assert_eq!(envelope.ordinary_effects(), &before.effects);
            assert!(envelope.source_effects().callback().kinds.is_unknown());
            assert!(envelope.source_effects().requires_world_barrier());
            assert_eq!(selected.facts(), before);
            assert_eq!(
                envelope.obligations(),
                [
                    ConditionalSourceObserverObligation::SelectedOperationApplicability,
                    ConditionalSourceObserverObligation::NormalContinuation,
                    ConditionalSourceObserverObligation::UnresolvedChannelCallbacks,
                    ConditionalSourceObserverObligation::LookupWorldAfterCallbacks,
                ]
            );
            let foreign_dialect = if version == tcl_dialect::TclVersion::V8_6 {
                InvocationDialect::for_version(tcl_dialect::TclVersion::V9_0)
            } else {
                InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6)
            };
            assert!(
                selected
                    .conditional_source_observer_envelope(foreign_dialect)
                    .is_none()
            );
        }
    }

    #[test]
    fn implicit_channel_source_envelope_refuses_other_handlers_channels_options_and_expansions() {
        assert!(envelope("puts", &[InvocationWord::Dynamic]).is_some());
        for arguments in [
            vec![],
            vec![InvocationWord::Expanded],
            vec![InvocationWord::Dynamic, InvocationWord::Dynamic],
            vec![InvocationWord::Literal("stdout"), InvocationWord::Dynamic],
            vec![
                InvocationWord::Literal("-nonewline"),
                InvocationWord::Dynamic,
            ],
        ] {
            assert!(envelope("puts", &arguments).is_none());
        }
        for head in ["unknown", "eval", "uplevel", "mystery", "list"] {
            assert!(envelope(head, &[InvocationWord::Dynamic]).is_none());
        }
        let dialect = InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
        let mut registry = crate::CommandRegistry::build_default();
        registry.insert(crate::CommandSpec {
            name: "unclassified_channel_handler",
            arity: crate::Arity::exact(1),
            ..crate::CommandSpec::DEFAULT
        });
        let arguments = [InvocationWord::Dynamic];
        let resolution = registry.resolve_structured_invocation(
            InvocationWords::structured(
                InvocationWord::Literal("unclassified_channel_handler"),
                &arguments,
            )
            .with_dialect(dialect),
            dialect.authoring_query(),
        );
        assert!(
            resolution
                .resolved()
                .unwrap()
                .conditional_source_observer_envelope(dialect)
                .is_none(),
            "an unclassified handler supplies no authored channel-write operation"
        );
    }

    #[test]
    fn implicit_channel_source_envelope_refuses_changed_source_operation_without_a_metadata_donor()
    {
        let dialect = InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
        let context = crate::model::ingress::static_context_for("tcl8.6");
        let arguments = [InvocationWord::Dynamic];
        let selected = context.commands().resolve_structured_invocation(
            InvocationWords::structured(InvocationWord::Literal("puts"), &arguments)
                .with_dialect(dialect),
            dialect.authoring_query(),
        );
        let mut changed = selected
            .resolved()
            .unwrap()
            .authored_source_descriptors()
            .command
            .clone();
        changed.semantic_operation = None;
        let mut registry = crate::CommandRegistry::build_default();
        registry.insert(changed);
        let resolution = registry.resolve_structured_invocation(
            InvocationWords::structured(InvocationWord::Literal("puts"), &arguments)
                .with_dialect(dialect),
            dialect.authoring_query(),
        );
        let selected = resolution.resolved().unwrap();
        assert!(selected.selected.spec.semantic_operation.is_none());
        assert!(
            selected
                .conditional_source_observer_envelope(dialect)
                .is_none()
        );
        assert!(selected.facts().effects.callback().kinds.is_unknown());
    }
}
