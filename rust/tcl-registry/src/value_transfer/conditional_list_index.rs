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

//! Conditional source bytes of one selected ordinary C list-index operand.

use tcl_dialect::{EscapeSyntax, IndexSyntax, ListParse, TclVersion};

use crate::native_compilation::{
    NativeBodyCompilation, NativeCompilationGrammar, SuccessfulHandlerSpec,
};
use crate::{ResolvedInvocation, SemanticOperationId, SourceIndexBounds};

use super::{AnalysisInputs, InvocationLayout, OperandId};

/// Conditional bytes selected from an independently supplied original literal
/// container. Original handler success, ordinary objects, no observers and
/// source correspondence remain separate premises. This grants no current
/// value, result representation, identity, store, Normal or Native authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConditionalSourceLiteralListIndex {
    container: OperandId,
    index_operand: OperandId,
    index: String,
    indices: IndexSyntax,
    list: ListParse,
    escapes: EscapeSyntax,
}

impl ConditionalSourceLiteralListIndex {
    /// Original post-head container operand, under the same selected layout.
    #[must_use]
    pub const fn container_operand(&self) -> OperandId {
        self.container
    }

    /// Original post-head operand containing the one literal scalar index.
    #[must_use]
    pub const fn index_operand(&self) -> OperandId {
        self.index_operand
    }

    /// Definite malformed ordinary-list bytes under the selected grammar.
    /// Unknown encoding, non-ASCII and NUL-bearing input remain unavailable;
    /// this does not classify an unavailable index or grant an error result.
    #[must_use]
    pub fn definitely_rejects_literal(&self, bytes: &[u8]) -> bool {
        bytes.is_ascii()
            && !bytes.contains(&0)
            && tcl_syntax::list::split_list_bytes_in(bytes, self.list, self.escapes).is_err()
    }

    /// Project only independently retained ASCII literal container bytes.
    /// Unknown input, malformed lists, unavailable index arithmetic or encoding
    /// abstain. A known empty element and a valid out-of-range index can both
    /// yield empty bytes; unavailable input never supplies that conclusion.
    #[must_use]
    pub fn project_literal(&self, bytes: Option<&[u8]>) -> Option<Vec<u8>> {
        let bytes = bytes?;
        if !bytes.is_ascii() || bytes.contains(&0) {
            return None;
        }
        let elements =
            tcl_syntax::list::split_list_bytes_in(bytes, self.list, self.escapes).ok()?;
        let index = tcl_cmd_core::index::resolve_opt_in(&self.index, elements.len(), self.indices)?;
        let Some(selected) = usize::try_from(index)
            .ok()
            .and_then(|index| elements.get(index))
        else {
            return Some(Vec::new());
        };
        (selected.is_ascii() && !selected.contains(&0)).then(|| selected.to_vec())
    }
}

fn closed_empty_world(descriptor: Option<crate::world_effect::WorldEffectDescriptor>) -> bool {
    use crate::world_effect::{WorldEffectDescriptor, WorldEffectDynamicFallback};
    let Some(descriptor) = descriptor else {
        return false;
    };
    let baseline = WorldEffectDescriptor::EMPTY;
    descriptor.composition == baseline.composition
        && descriptor.static_footprint.accesses.is_empty()
        && descriptor.static_footprint.callback == baseline.static_footprint.callback
        && descriptor.resolver.is_none()
        && descriptor.source_resolver.is_none()
        && matches!(
            descriptor.dynamic_fallback,
            WorldEffectDynamicFallback::ConservativeUnknownInvocation
        )
}

fn closed_empty_transitions(
    descriptor: Option<crate::state_transition::StateTransitionDescriptor>,
) -> bool {
    use crate::state_transition::StateTransitionDescriptor;
    let Some(descriptor) = descriptor else {
        return false;
    };
    let baseline = StateTransitionDescriptor::EMPTY;
    descriptor.composition == baseline.composition
        && descriptor.resolver.is_none()
        && descriptor.success_resolver.is_none()
        && descriptor.logical_source_resolver.is_none()
        && descriptor.argument_shape == baseline.argument_shape
        && descriptor.dynamic_widening.is_empty()
        && descriptor.effect_coverage.is_empty()
        && descriptor.commit == baseline.commit
}

fn closed_single_index_effects(selected: &ResolvedInvocation<'_, '_>) -> bool {
    let world = selected.semantics.world_effects;
    let transitions = selected.semantics.state_transitions;
    closed_empty_world(selected.selected.spec.world_effects)
        && closed_empty_world(world.command)
        && world.subcommand.is_none()
        && world.form.is_none()
        && closed_empty_transitions(selected.selected.spec.state_transitions)
        && closed_empty_transitions(transitions.command)
        && transitions.subcommand.is_none()
        && transitions.form.is_none()
}

impl ResolvedInvocation<'_, '_> {
    /// Select one literal scalar-index source algebra from the exact C profile.
    /// This cannot complete an unknown container, certify an original handler,
    /// promote a source expression or issue an executable evaluation answer.
    #[must_use]
    pub fn conditional_source_single_literal_list_index(
        &self,
        input: &dyn AnalysisInputs,
    ) -> Option<ConditionalSourceLiteralListIndex> {
        let profile = input.context().profile?;
        let version = TclVersion::from_profile(profile)?;
        let arguments = self.words.arguments();
        let dialect = arguments.dialect()?;
        let index_syntax = dialect.index_syntax()?;
        let source = self.authored_source_index_bounds()?;
        let view = input.invocation();
        let facts = self.facts();
        let native = self.semantics.native_compilation?;
        let raw_native = self.selected.spec.native_compilation?;
        let operation = SemanticOperationId::Intrinsic(crate::IntrinsicId::ListIndex);
        let representation = crate::RepresentationEffect::CoerceOrdinaryListIndices {
            operand: 0,
            indices_from: 1,
        };
        if !profile.is_tcl_release()
            || profile.runtime_core_family() != Some(tcl_dialect::model::Family::Tcl)
            || dialect.family() != Some(tcl_dialect::model::Family::Tcl)
            || dialect.tcl_version != Some(version)
            || dialect.core_point != profile.core_point
            || dialect.numbers != profile.grammar.numbers
            || input.context().grammar != profile.grammar
            || dialect.lexer_grammar != profile.grammar
            || dialect.word_values.list != profile.grammar.list_parse
            || profile.index_syntax() != Some(index_syntax)
            || self.dialect.as_ref()?.core != profile.surface_query().core
            || source.operation != SourceIndexBounds::ListIndex
            || source.arguments != (0..2)
            || arguments.exact_argv_len() != Some(2)
            || arguments.len() != 2
            || self.selected.sub.is_some()
            || self.selected.form.is_some()
            || self.selected.instance
            || !self.selected.spec.side_effects.is_empty()
            || !self.semantics.side_effects.is_empty()
            || self.selected.spec.representation_effect != Some(representation)
            || self.semantics.representation_effect != representation
            || self.semantics.argument_offset != 0
            || view.argument_offset != 0
            || view.canonical_command != self.canonical_command
            || view.subcommand.is_some()
            || view.form.is_some()
            || view.layout != InvocationLayout::Source
            || view.operands.len() != 2
            || view.operands.iter().any(|word| {
                matches!(
                    word.kind,
                    crate::InvocationWordKind::Expanded | crate::InvocationWordKind::Opaque
                )
            })
            || facts.arity_accepts_frozen_arguments() != Some(true)
            || !closed_single_index_effects(self)
            || !facts.effects.accesses().is_empty()
            || facts.effects.callback() != crate::world_effect::CallbackEffect::NONE
            || facts.operation != operation
            || native.grammar != NativeCompilationGrammar::ListIndex
            || native.operation != operation
            || native.body != NativeBodyCompilation::Inherit
            || raw_native.grammar != native.grammar
            || raw_native.operation != native.operation
            || raw_native.body != native.body
            || self.semantics.successful_handler != Some(SuccessfulHandlerSpec::Leaf)
            || self.semantics.body_kind != crate::body_kind::BodyKind::Plain
            || self.semantics.body_interpreter != crate::body_kind::BodyInterpreter::Current
            || self.semantics.frame_effect.is_some()
            || self.semantics.body_execution.is_some()
        {
            return None;
        }
        let index = arguments.literal_at(1)?;
        if !index.is_ascii() || index.contains('\0') {
            return None;
        }
        // The shared scalar-index owner deliberately refuses grouped paths.
        // Validation at zero may abstain on an extreme end-relative spelling;
        // every projected container is resolved independently again below.
        tcl_cmd_core::index::resolve_opt_in(index, 0, index_syntax)?;
        self.semantics
            .representation_effect
            .conditional_source_list_index_coercion(arguments, 0)?;
        Some(ConditionalSourceLiteralListIndex {
            container: OperandId(0),
            index_operand: OperandId(1),
            index: index.to_owned(),
            indices: index_syntax,
            list: dialect.word_values.list,
            escapes: dialect.lexer_grammar.escapes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value_transfer::LiteralInputs;
    use crate::{
        CommandRegistry, InvocationArguments, InvocationDialect, InvocationWord, InvocationWords,
    };

    fn relation(
        environment: &str,
        arguments: &[InvocationWord<'_>],
    ) -> Option<ConditionalSourceLiteralListIndex> {
        let profile = crate::model::ingress::resolve_known_environment(environment)?.unit_profile();
        let registry = CommandRegistry::build_default().project_for_profile(profile);
        let selected = registry
            .resolve_structured_invocation(
                InvocationWords::from_arguments(
                    InvocationWord::Literal("lindex"),
                    InvocationArguments::structured(arguments)
                        .with_dialect(InvocationDialect::of_profile(profile)),
                ),
                Some(profile.surface_query()),
            )
            .resolved()?;
        let texts: Vec<_> = arguments
            .iter()
            .map(|word| match word {
                InvocationWord::Literal(text) => *text,
                _ => "unknown",
            })
            .collect();
        let input = LiteralInputs::new(selected.canonical_command, None, &texts, Some(profile));
        selected.conditional_source_single_literal_list_index(&input)
    }

    #[test]
    fn source_single_list_index_projects_zero_and_one_without_representation() {
        for version in TclVersion::ALL {
            for (index, expected) in [("0", b"1".as_slice()), ("1", b"2".as_slice())] {
                let descriptor = relation(
                    version.dialect_profile_name(),
                    &[InvocationWord::Dynamic, InvocationWord::Literal(index)],
                )
                .expect("selected original C single-index source shape");
                assert_eq!(descriptor.container_operand(), OperandId(0));
                assert_eq!(descriptor.index_operand(), OperandId(1));
                assert_eq!(
                    descriptor.project_literal(Some(b"1 2")),
                    Some(expected.to_vec())
                );
                assert_eq!(descriptor.project_literal(None), None);
                assert_eq!(descriptor.project_literal(Some(b"{} {}")), Some(Vec::new()));
                assert_eq!(descriptor.project_literal(Some(b"{")), None);
                assert_eq!(descriptor.project_literal(Some(b"\xff 2")), None);
                assert_eq!(descriptor.project_literal(Some(b"a\0b 2")), None);
            }
            for index in ["-1", "2"] {
                let descriptor = relation(
                    version.dialect_profile_name(),
                    &[InvocationWord::Dynamic, InvocationWord::Literal(index)],
                )
                .expect("valid scalar index");
                assert_eq!(descriptor.project_literal(Some(b"1 2")), Some(Vec::new()));
            }
        }
    }

    #[test]
    fn source_single_list_index_refuses_unavailable_index_shapes() {
        for version in TclVersion::ALL {
            for index in ["", "bad", "0 1", "{1}"] {
                assert!(
                    relation(
                        version.dialect_profile_name(),
                        &[InvocationWord::Dynamic, InvocationWord::Literal(index),]
                    )
                    .is_none(),
                    "{version:?}: {index}"
                );
            }
            for unknown in [
                InvocationWord::Dynamic,
                InvocationWord::Expanded,
                InvocationWord::Opaque,
            ] {
                assert!(
                    relation(
                        version.dialect_profile_name(),
                        &[InvocationWord::Dynamic, unknown,]
                    )
                    .is_none()
                );
            }
            for arguments in [
                &[InvocationWord::Dynamic][..],
                &[
                    InvocationWord::Dynamic,
                    InvocationWord::Literal("0"),
                    InvocationWord::Literal("1"),
                ][..],
                &[InvocationWord::Expanded, InvocationWord::Literal("1")][..],
                &[InvocationWord::Opaque, InvocationWord::Literal("1")][..],
            ] {
                assert!(relation(version.dialect_profile_name(), arguments).is_none());
            }
        }
    }

    #[test]
    fn source_single_list_index_keeps_release_and_profile_boundaries() {
        for environment in ["jim", "tcl", "tk", "f5-irules"] {
            assert!(
                relation(
                    environment,
                    &[InvocationWord::Dynamic, InvocationWord::Literal("1"),]
                )
                .is_none()
            );
        }
        for version in TclVersion::ALL {
            let descriptor = relation(
                version.dialect_profile_name(),
                &[InvocationWord::Dynamic, InvocationWord::Literal("0+1")],
            );
            assert_eq!(descriptor.is_some(), version != TclVersion::V8_4);
            if let Some(descriptor) = descriptor {
                assert_eq!(
                    descriptor.project_literal(Some(b"1 2")),
                    Some(b"2".to_vec())
                );
            }
        }
    }

    #[test]
    fn source_single_list_index_rejects_changed_selected_operation_and_input() {
        let profile = crate::model::ingress::resolve_known_environment("tcl9.0")
            .unwrap()
            .unit_profile();
        let mut registry = CommandRegistry::build_default().project_for_profile(profile);
        let mut spec = registry.get("lindex").unwrap().clone();
        spec.semantic_operation = Some(SemanticOperationId::Invoke);
        registry.insert(spec);
        let words = [InvocationWord::Dynamic, InvocationWord::Literal("1")];
        let selected = registry
            .resolve_structured_invocation(
                InvocationWords::from_arguments(
                    InvocationWord::Literal("lindex"),
                    InvocationArguments::structured(&words)
                        .with_dialect(InvocationDialect::of_profile(profile)),
                ),
                Some(profile.surface_query()),
            )
            .resolved()
            .unwrap();
        assert_eq!(selected.facts().operation, SemanticOperationId::Invoke);
        let input = LiteralInputs::new(
            selected.canonical_command,
            None,
            &["unknown", "1"],
            Some(profile),
        );
        assert!(
            selected
                .conditional_source_single_literal_list_index(&input)
                .is_none()
        );
        let registry = CommandRegistry::build_default().project_for_profile(profile);
        let selected = registry
            .resolve_structured_invocation(
                InvocationWords::from_arguments(
                    InvocationWord::Literal("lindex"),
                    InvocationArguments::structured(&words)
                        .with_dialect(InvocationDialect::of_profile(profile)),
                ),
                Some(profile.surface_query()),
            )
            .resolved()
            .unwrap();
        for changed in [
            None,
            Some(
                crate::model::ingress::resolve_known_environment("tcl8.6")
                    .unwrap()
                    .unit_profile(),
            ),
        ] {
            let input =
                LiteralInputs::new(selected.canonical_command, None, &["unknown", "1"], changed);
            assert!(
                selected
                    .conditional_source_single_literal_list_index(&input)
                    .is_none()
            );
        }
    }
    #[derive(Debug, Clone, Copy)]
    enum IndexEffectMutation {
        DirectWorldWrite,
        VariableCellTransition,
        CommandBindingTransition,
    }

    fn apply_index_effect_mutation(spec: &mut crate::CommandSpec, mutation: IndexEffectMutation) {
        use crate::state_transition::{
            StateTransition, StateTransitionDescriptor, StateTransitionDomain,
            StateTransitionWidening, StateTransitions, TransitionSubject,
        };
        use crate::world_effect::{
            EffectAccessMode, StaticEffectAccess, StaticEffectFootprint, StaticInterpreterScope,
            StaticNamespaceScope, StaticSubjectScope, WorldEffectDescriptor, WorldStateDomain,
        };
        const DIRECT_WRITE: WorldEffectDescriptor = WorldEffectDescriptor {
            static_footprint: StaticEffectFootprint {
                accesses: &[StaticEffectAccess::new(
                    WorldStateDomain::VariableStore,
                    EffectAccessMode::Write,
                    StaticInterpreterScope::Current,
                    StaticNamespaceScope::Current,
                    StaticSubjectScope::Wildcard,
                )],
                ..StaticEffectFootprint::EMPTY
            },
            ..WorldEffectDescriptor::EMPTY
        };
        fn variable_transition(_: InvocationArguments<'_>) -> StateTransitions {
            let mut transitions = StateTransitions::default();
            transitions.push(StateTransition::Widen(StateTransitionWidening {
                domains: vec![StateTransitionDomain::VariableCells],
                subject: TransitionSubject::Literal("out".to_owned()),
            }));
            transitions
        }
        fn command_transition(_: InvocationArguments<'_>) -> StateTransitions {
            StateTransitions::unknown_invocation()
        }
        match mutation {
            IndexEffectMutation::DirectWorldWrite => spec.world_effects = Some(DIRECT_WRITE),
            IndexEffectMutation::VariableCellTransition => {
                spec.state_transitions = Some(StateTransitionDescriptor {
                    resolver: Some(variable_transition),
                    ..StateTransitionDescriptor::EMPTY
                });
            }
            IndexEffectMutation::CommandBindingTransition => {
                spec.state_transitions = Some(StateTransitionDescriptor {
                    resolver: Some(command_transition),
                    ..StateTransitionDescriptor::EMPTY
                });
            }
        }
    }

    #[test]
    fn source_single_list_index_refuses_declared_noncallback_effects_and_transitions() {
        use crate::state_transition::StateTransitionDomain;
        let profile = crate::model::ingress::resolve_known_environment("tcl9.0")
            .unwrap()
            .unit_profile();
        for mutation in [
            IndexEffectMutation::DirectWorldWrite,
            IndexEffectMutation::VariableCellTransition,
            IndexEffectMutation::CommandBindingTransition,
        ] {
            let mut registry = CommandRegistry::build_default().project_for_profile(profile);
            let mut spec = registry.get("lindex").unwrap().clone();
            apply_index_effect_mutation(&mut spec, mutation);
            registry.insert(spec);
            let words = [InvocationWord::Dynamic, InvocationWord::Literal("1")];
            let selected = registry
                .resolve_structured_invocation(
                    InvocationWords::from_arguments(
                        InvocationWord::Literal("lindex"),
                        InvocationArguments::structured(&words)
                            .with_dialect(InvocationDialect::of_profile(profile)),
                    ),
                    Some(profile.surface_query()),
                )
                .resolved()
                .unwrap();
            let facts = selected.facts();
            match mutation {
                IndexEffectMutation::DirectWorldWrite => {
                    assert!(!facts.effects.requires_world_barrier(), "{mutation:?}");
                    assert!(!facts.effects.accesses().is_empty(), "{mutation:?}");
                }
                IndexEffectMutation::VariableCellTransition => {
                    let transitions = facts
                        .state_transitions
                        .declared()
                        .expect("selected variable transition");
                    assert!(
                        transitions.widens(StateTransitionDomain::VariableCells),
                        "{mutation:?}"
                    );
                    assert!(!transitions.touches_command_bindings(), "{mutation:?}");
                    assert!(!facts.effects.requires_world_barrier(), "{mutation:?}");
                }
                IndexEffectMutation::CommandBindingTransition => {
                    let transitions = facts
                        .state_transitions
                        .declared()
                        .expect("selected command transition");
                    assert!(transitions.touches_command_bindings(), "{mutation:?}");
                    assert!(
                        facts.effects.requires_world_barrier(),
                        "{mutation:?}: command trace callbacks"
                    );
                }
            }
            let input = LiteralInputs::new(
                selected.canonical_command,
                None,
                &["unknown", "1"],
                Some(profile),
            );
            assert!(
                selected
                    .conditional_source_single_literal_list_index(&input)
                    .is_none(),
                "{mutation:?}: selected source contract must decline"
            );
        }
    }

    #[test]
    fn source_single_list_index_separates_definite_malformed_from_unknown_input() {
        for version in TclVersion::ALL {
            for index in ["0", "1"] {
                let descriptor = relation(
                    version.dialect_profile_name(),
                    &[InvocationWord::Dynamic, InvocationWord::Literal(index)],
                )
                .expect("selected C literal index with unknown container");
                for malformed in [b"{".as_slice(), b"\"", b"{x}tail"] {
                    assert!(descriptor.definitely_rejects_literal(malformed));
                    assert_eq!(descriptor.project_literal(Some(malformed)), None);
                }
                for unavailable_or_valid in [b"".as_slice(), b"1 2", b"{}", b"\xff{", b"{\0"] {
                    assert!(!descriptor.definitely_rejects_literal(unavailable_or_valid));
                }
                assert_eq!(descriptor.project_literal(None), None);
            }
        }
    }

    #[test]
    fn source_single_list_index_requires_raw_and_effective_stock_representation() {
        for version in TclVersion::ALL {
            let profile =
                crate::model::ingress::resolve_known_environment(version.dialect_profile_name())
                    .unwrap()
                    .unit_profile();
            assert!(
                relation(
                    version.dialect_profile_name(),
                    &[InvocationWord::Dynamic, InvocationWord::Literal("1"),]
                )
                .is_some()
            );
            let baseline = crate::RepresentationEffect::CoerceOrdinaryListIndices {
                operand: 0,
                indices_from: 1,
            };
            let altered = crate::RepresentationEffect::CoerceOrdinaryListIndices {
                operand: 1,
                indices_from: 1,
            };
            for override_at in 0..3 {
                let mut registry = CommandRegistry::build_default().project_for_profile(profile);
                let mut spec = registry.get("lindex").unwrap().clone();
                assert_eq!(spec.representation_effect, Some(baseline));
                if override_at != 1 {
                    spec.representation_effect = Some(altered);
                }
                registry.insert(spec);
                let words = [InvocationWord::Dynamic, InvocationWord::Literal("1")];
                let mut selected = registry
                    .resolve_structured_invocation(
                        InvocationWords::from_arguments(
                            InvocationWord::Literal("lindex"),
                            InvocationArguments::structured(&words)
                                .with_dialect(InvocationDialect::of_profile(profile)),
                        ),
                        Some(profile.surface_query()),
                    )
                    .resolved()
                    .unwrap();
                assert!(
                    altered
                        .conditional_source_list_index_coercion(selected.words.arguments(), 0)
                        .is_some()
                );
                match override_at {
                    0 => {
                        assert_eq!(selected.selected.spec.representation_effect, Some(altered));
                        assert_eq!(selected.semantics.representation_effect, altered);
                    }
                    1 => {
                        assert_eq!(selected.selected.spec.representation_effect, Some(baseline));
                        selected.semantics.representation_effect = altered;
                    }
                    _ => {
                        assert_eq!(selected.selected.spec.representation_effect, Some(altered));
                        selected.semantics.representation_effect = baseline;
                    }
                }
                let input = LiteralInputs::new(
                    selected.canonical_command,
                    None,
                    &["unknown", "1"],
                    Some(profile),
                );
                assert!(
                    selected
                        .conditional_source_single_literal_list_index(&input)
                        .is_none(),
                    "{version:?}: representation override {override_at}"
                );
            }
        }
    }
}
