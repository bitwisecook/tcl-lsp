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

//! Conditional ordinary-container effect of an original nonempty list append.

use tcl_dialect::{EscapeSyntax, ListParse, TclVersion};

use crate::hooks::LoweringHookId;
use crate::native_compilation::{
    NativeAppendKind, NativeBodyCompilation, NativeCompilationGrammar, SuccessfulHandlerSpec,
};
use crate::native_lowering::{CellUpdate, NativeLowering};
use crate::side_effects::{SideEffect, SideEffectTarget};
use crate::{ResolvedInvocation, SemanticOperationId, TclType};

use super::{
    AnalysisInputs, CommandSemantics, DerivedSemantics, InvocationLayout, PlanAnswer,
    ResolvedSemantics, TargetId,
};

pub(crate) const CONDITIONAL_LIST_APPEND_LEGACY_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Variable,
    reads: true,
    writes: true,
    ..SideEffect::DEFAULT
}];

/// Exact existing stock traits shared with the registry declaration.
pub(crate) const CONDITIONAL_LIST_APPEND_TRAITS: crate::Traits = crate::Traits::FRAMELESS_RUNTIME
    .union(crate::Traits::NOT_PROC_FACTORY)
    .union(crate::Traits::BYTE_COMPILED)
    .union(crate::Traits::READS_BEFORE_WRITE)
    .union(crate::Traits::FIRST_ARG_VARNAME)
    .union(crate::Traits::UNCONDITIONAL_VARIABLE_WRITE);

fn direct_phase_layout_is_supported(selected: &ResolvedInvocation<'_, '_>) -> bool {
    let raw = selected.selected.spec;
    let effective = &selected.semantics;
    raw.traits == CONDITIONAL_LIST_APPEND_TRAITS
        && effective.traits == CONDITIONAL_LIST_APPEND_TRAITS
        && raw.frame_effect.is_none()
        && effective.frame_effect.is_none()
        && raw.body_execution.is_none()
        && effective.body_execution.is_none()
        && raw.body_kind == crate::body_kind::BodyKind::Plain
        && effective.body_kind == raw.body_kind
        && raw.body_interpreter == crate::body_kind::BodyInterpreter::Current
        && effective.body_interpreter == raw.body_interpreter
}

fn direct_phase_execution_is_stock(
    selected: &ResolvedInvocation<'_, '_>,
    native: crate::native_compilation::NativeCompilationSpec,
) -> bool {
    let Some(raw) = selected.selected.spec.native_compilation else {
        return false;
    };
    raw == native
        && native.grammar == NativeCompilationGrammar::VariableAppend(NativeAppendKind::List)
        && native.operation
            == SemanticOperationId::StructuredLowering(LoweringHookId::AppendOrLappend)
        && native.body == NativeBodyCompilation::Inherit
        && selected.selected.spec.successful_handler
            == Some(SuccessfulHandlerSpec::VariableOperands)
        && selected.semantics.successful_handler == Some(SuccessfulHandlerSpec::VariableOperands)
}

/// Conditional direct kernel after all original operands have evaluated.
/// The earlier captured receiver identifies the CURRENT target cell read at
/// this phase. Original successful C handling, ordinary objects, unobserved
/// cells and absent callbacks/aliases remain independent caller premises.
/// This is no whole-command quietness, body entry or execution receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConditionalSourceListAppendDirectPhase {
    target: TargetId,
    values_from: usize,
    values_end: usize,
    lookup_preserved: bool,
}

impl ConditionalSourceListAppendDirectPhase {
    /// Earlier receiver operand whose cell is read after all operand effects.
    /// This neither captures the old cell contents nor proves a physical read.
    #[must_use]
    pub const fn current_target(self) -> TargetId {
        self.target
    }

    /// Original appended value operands, already evaluated before this phase.
    #[must_use]
    pub fn evaluated_value_operands(self) -> std::ops::Range<usize> {
        self.values_from..self.values_end
    }

    /// Lookup preservation only under this phase's independent premises.
    /// Ordered operand effects and later/unknown phases retain their barriers.
    #[must_use]
    pub const fn conditionally_preserves_lookup(self) -> bool {
        self.lookup_preserved
    }
}

/// A conditional List requirement and List store/result for one target
/// container. The actual original handler must succeed over ordinary objects
/// and an unobserved cell. Aliases, callbacks, abstract-list implementations,
/// source correspondence and successful completion remain caller obligations.
/// This grants no element forwarding, physical identity or execution authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConditionalSourceListAppend {
    target: TargetId,
    appended_operands: usize,
    prior: TclType,
    stored: TclType,
    result: TclType,
    list: ListParse,
    escapes: EscapeSyntax,
    direct_phase: ConditionalSourceListAppendDirectPhase,
}

impl ConditionalSourceListAppend {
    /// Separately named structural-kernel contract; not a broad effect grant.
    #[must_use]
    pub const fn conditional_direct_phase(self) -> ConditionalSourceListAppendDirectPhase {
        self.direct_phase
    }

    /// Original target coordinate selected by the cell-update structure.
    #[must_use]
    pub const fn target(self) -> TargetId {
        self.target
    }

    /// Exact number of original appended value operands; always nonzero.
    #[must_use]
    pub const fn appended_operand_count(self) -> usize {
        self.appended_operands
    }

    /// An existing ordinary prior value is consumed as a list. This is an
    /// operation requirement, never evidence that the prior already is List.
    /// It does not certify physical coercion of the prior object: the handler
    /// may copy it before conversion. No new input-conversion finding follows.
    #[must_use]
    pub const fn prior_representation_requirement(self) -> TclType {
        self.prior
    }

    /// Conditional representation of the stored container, not its elements.
    #[must_use]
    pub const fn stored_representation(self) -> TclType {
        self.stored
    }

    /// Conditional representation of the successful handler's result.
    #[must_use]
    pub const fn result_representation(self) -> TclType {
        self.result
    }

    /// Definitely malformed ASCII prior-list bytes under the selected policy.
    /// Unknown encoding, non-ASCII and NUL-bearing values remain conditional.
    #[must_use]
    pub fn definitely_rejects_literal(self, bytes: &[u8]) -> bool {
        bytes.is_ascii()
            && !bytes.contains(&0)
            && tcl_syntax::list::split_list_bytes_in(bytes, self.list, self.escapes).is_err()
    }
}

impl ResolvedInvocation<'_, '_> {
    /// Selected ordinary C Tcl list-append relation, conditional on successful
    /// original handling and independently established unobserved-cell premises.
    /// The description cannot establish Native dispatch, completion or a store.
    #[must_use]
    pub fn conditional_source_nonempty_list_append(
        &self,
        input: &dyn AnalysisInputs,
    ) -> Option<ConditionalSourceListAppend> {
        let profile = input.context().profile?;
        let version = TclVersion::from_profile(profile)?;
        let arguments = self.words.arguments();
        let dialect = arguments.dialect()?;
        let view = input.invocation();
        let count = arguments.exact_argv_len()?;
        let facts = self.facts();
        let native = self.semantics.native_compilation?;
        let representation = crate::RepresentationEffect::copy_on_write_container(0, 2);
        let ResolvedSemantics::Derived(DerivedSemantics::CellUpdate(update)) = self.semantics.value
        else {
            return None;
        };
        if !profile.is_tcl_release()
            || profile.runtime_core_family() != Some(tcl_dialect::model::Family::Tcl)
            || dialect.family() != Some(tcl_dialect::model::Family::Tcl)
            || dialect.tcl_version != Some(version)
            || input.context().grammar != profile.grammar
            || dialect.lexer_grammar != profile.grammar
            || dialect.word_values.list != profile.grammar.list_parse
            || self.dialect.as_ref()?.core != profile.surface_query().core
            || update.update != CellUpdate::ListAppend
            || self.selected.spec.native_lowering
                != Some(NativeLowering::CellReadModifyWrite(CellUpdate::ListAppend))
            || self.selected.sub.is_some()
            || self.selected.form.is_some()
            || self.selected.instance
            || !direct_phase_layout_is_supported(self)
            || self.selected.spec.side_effects != CONDITIONAL_LIST_APPEND_LEGACY_EFFECTS
            || self.semantics.side_effects != CONDITIONAL_LIST_APPEND_LEGACY_EFFECTS
            || self.selected.spec.representation_effect != Some(representation)
            || self.semantics.representation_effect != representation
            || self.semantics.argument_offset != 0
            || view.argument_offset != 0
            || view.canonical_command != self.canonical_command
            || view.subcommand.is_some()
            || view.form.is_some()
            || view.layout != InvocationLayout::Source
            || count < 2
            || count != arguments.len()
            || count != view.operands.len()
            || view.operands.iter().any(|word| {
                matches!(
                    word.kind,
                    crate::InvocationWordKind::Expanded | crate::InvocationWordKind::Opaque
                )
            })
            || facts.arity_accepts_frozen_arguments() != Some(true)
            || !facts.arg_roles_complete
            || !direct_phase_execution_is_stock(self, native)
            || facts.operation != native.operation
            || self.selected.spec.state_transitions.is_some()
            || self.semantics.state_transitions.is_declared()
            || self.selected.spec.world_effects.is_some()
            || self.semantics.world_effects.is_declared()
        {
            return None;
        }
        let PlanAnswer::CellReadModifyWrite {
            target,
            operation: CellUpdate::ListAppend,
            amount: Some(values_from),
            ..
        } = update.structure(input)
        else {
            return None;
        };
        let appended_operands = count.checked_sub(values_from.0)?;
        if appended_operands == 0 || target.0.0 >= values_from.0 {
            return None;
        }
        Some(ConditionalSourceListAppend {
            target,
            appended_operands,
            prior: TclType::List,
            stored: TclType::List,
            result: TclType::List,
            list: dialect.word_values.list,
            escapes: dialect.lexer_grammar.escapes,
            direct_phase: ConditionalSourceListAppendDirectPhase {
                target,
                values_from: values_from.0,
                values_end: count,
                lookup_preserved: true,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value_transfer::LiteralInputs;
    use crate::{
        CommandRegistry, CommandSpec, InvocationArguments, InvocationDialect, InvocationWord,
        InvocationWords,
    };

    const EXTRA_EFFECTS: &[SideEffect] = &[
        SideEffect {
            target: SideEffectTarget::Variable,
            reads: true,
            writes: true,
            ..SideEffect::DEFAULT
        },
        SideEffect {
            writes: true,
            ..SideEffect::DEFAULT
        },
    ];

    #[derive(Debug, Clone, Copy)]
    enum SchemaChange {
        None,
        Shadow,
        RawExtra,
        EffectiveExtra,
        FormHidesRawExtra,
        EffectiveOperation,
    }

    fn selected_relation(
        environment: &str,
        arguments: &[&str],
        change: SchemaChange,
    ) -> Option<ConditionalSourceListAppend> {
        let profile = crate::model::ingress::resolve_known_environment(environment)?.unit_profile();
        let mut registry = CommandRegistry::build_default().project_for_profile(profile);
        match change {
            SchemaChange::Shadow => {
                let surface = registry.get("lappend")?.surface;
                registry.insert(CommandSpec {
                    name: "lappend",
                    surface,
                    ..CommandSpec::DEFAULT
                });
            }
            SchemaChange::RawExtra | SchemaChange::FormHidesRawExtra => {
                let mut spec = registry.get("lappend")?.clone();
                spec.side_effects = EXTRA_EFFECTS;
                if matches!(change, SchemaChange::FormHidesRawExtra) {
                    const FORM: &[crate::forms::CommandForm] = &[crate::forms::CommandForm {
                        name: "hidden-raw-effects",
                        arity: crate::Arity::at_least(2),
                        side_effects: Some(CONDITIONAL_LIST_APPEND_LEGACY_EFFECTS),
                        ..crate::forms::CommandForm::DEFAULT
                    }];
                    spec.command_forms = FORM;
                }
                registry.insert(spec);
            }
            SchemaChange::EffectiveOperation => {
                let mut spec = registry.get("lappend")?.clone();
                spec.semantic_operation = Some(SemanticOperationId::Invoke);
                registry.insert(spec);
            }
            SchemaChange::None | SchemaChange::EffectiveExtra => {}
        }
        let words: Vec<_> = arguments
            .iter()
            .copied()
            .map(InvocationWord::Literal)
            .collect();
        let arguments_view = InvocationArguments::structured(&words)
            .with_dialect(InvocationDialect::of_profile(profile));
        let mut selected = registry
            .resolve_structured_invocation(
                InvocationWords::from_arguments(InvocationWord::Literal("lappend"), arguments_view),
                Some(profile.surface_query()),
            )
            .resolved()?;
        match change {
            SchemaChange::EffectiveExtra => selected.semantics.side_effects = EXTRA_EFFECTS,
            SchemaChange::FormHidesRawExtra => {
                assert!(selected.selected.form.is_some());
                assert_eq!(selected.selected.spec.side_effects, EXTRA_EFFECTS);
                assert_eq!(
                    selected.semantics.side_effects,
                    CONDITIONAL_LIST_APPEND_LEGACY_EFFECTS
                );
            }
            SchemaChange::EffectiveOperation => {
                assert_eq!(selected.facts().operation, SemanticOperationId::Invoke);
                assert_eq!(
                    selected.semantics.native_compilation.unwrap().operation,
                    SemanticOperationId::StructuredLowering(LoweringHookId::AppendOrLappend)
                );
            }
            SchemaChange::Shadow => {
                assert!(
                    selected.selected.spec.native_lowering.is_none(),
                    "selected generic shadow"
                );
                assert!(
                    matches!(selected.semantics.value, ResolvedSemantics::None),
                    "selected generic shadow has no derived handler"
                );
            }
            SchemaChange::RawExtra => {
                assert_eq!(
                    selected.selected.spec.side_effects, EXTRA_EFFECTS,
                    "selected raw extra-effect descriptor"
                );
            }
            SchemaChange::None => {}
        }
        let input = LiteralInputs::new(selected.canonical_command, None, arguments, Some(profile));
        selected.conditional_source_nonempty_list_append(&input)
    }

    #[test]
    fn nonempty_c_list_append_describes_only_the_target_container() {
        for version in TclVersion::ALL {
            for arguments in [&["out", "5"][..], &["out", "5", "2.5"][..]] {
                let relation = selected_relation(
                    version.dialect_profile_name(),
                    arguments,
                    SchemaChange::None,
                )
                .expect("selected ordinary nonempty C handler");
                assert_eq!(relation.target(), TargetId(super::super::OperandId(0)));
                assert_eq!(relation.appended_operand_count(), arguments.len() - 1);
                let phase = relation.conditional_direct_phase();
                assert_eq!(phase.current_target(), relation.target());
                assert_eq!(phase.evaluated_value_operands(), 1..arguments.len());
                assert!(phase.conditionally_preserves_lookup());
                assert_eq!(relation.prior_representation_requirement(), TclType::List);
                assert_eq!(relation.stored_representation(), TclType::List);
                assert_eq!(relation.result_representation(), TclType::List);
                for invalid in [b"{".as_slice(), b"\"", b"{x}tail"] {
                    assert!(relation.definitely_rejects_literal(invalid));
                }
                for conditional in [b"".as_slice(), b"5", b"5 2.5", b"\xff{", b"{\0", b"\"\0"] {
                    assert!(!relation.definitely_rejects_literal(conditional));
                }
            }
        }
    }

    #[test]
    fn list_append_refuses_no_value_and_unaudited_profiles() {
        for version in TclVersion::ALL {
            assert!(
                selected_relation(version.dialect_profile_name(), &["out"], SchemaChange::None)
                    .is_none()
            );
            assert!(
                selected_relation(version.dialect_profile_name(), &[], SchemaChange::None)
                    .is_none()
            );
        }
        for environment in ["jim", "tcl", "f5-irules", "tk"] {
            assert!(selected_relation(environment, &["out", "5"], SchemaChange::None).is_none());
        }
    }

    #[test]
    fn list_append_requires_the_raw_and_effective_selected_contract() {
        for change in [
            SchemaChange::Shadow,
            SchemaChange::RawExtra,
            SchemaChange::EffectiveExtra,
            SchemaChange::FormHidesRawExtra,
            SchemaChange::EffectiveOperation,
        ] {
            assert!(
                selected_relation("tcl9.0", &["out", "5"], change).is_none(),
                "{change:?}: selected source contract must decline"
            );
        }
    }

    #[test]
    fn list_append_refuses_unknown_expansion_and_mismatched_input_policy() {
        let profile = crate::model::ingress::resolve_known_environment("tcl9.0")
            .unwrap()
            .unit_profile();
        let registry = CommandRegistry::build_default().project_for_profile(profile);
        for last in [InvocationWord::Expanded, InvocationWord::Opaque] {
            let words = [InvocationWord::Literal("out"), last];
            let selected = registry.resolve_structured_invocation(
                InvocationWords::from_arguments(
                    InvocationWord::Literal("lappend"),
                    InvocationArguments::structured(&words)
                        .with_dialect(InvocationDialect::of_profile(profile)),
                ),
                Some(profile.surface_query()),
            );
            if let Some(selected) = selected.resolved() {
                let input = LiteralInputs::new(
                    selected.canonical_command,
                    None,
                    &["out", "5"],
                    Some(profile),
                );
                assert!(
                    selected
                        .conditional_source_nonempty_list_append(&input)
                        .is_none()
                );
            }
        }
        let words = [InvocationWord::Literal("out"), InvocationWord::Literal("5")];
        let selected = registry
            .resolve_structured_invocation(
                InvocationWords::from_arguments(
                    InvocationWord::Literal("lappend"),
                    InvocationArguments::structured(&words)
                        .with_dialect(InvocationDialect::of_profile(profile)),
                ),
                Some(profile.surface_query()),
            )
            .resolved()
            .unwrap();
        let mut changed_policy = InvocationDialect::of_profile(profile);
        changed_policy.word_values.list = ListParse::Lenient;
        let changed = registry
            .resolve_structured_invocation(
                InvocationWords::from_arguments(
                    InvocationWord::Literal("lappend"),
                    InvocationArguments::structured(&words).with_dialect(changed_policy),
                ),
                Some(profile.surface_query()),
            )
            .resolved()
            .unwrap();
        let input = LiteralInputs::new(
            changed.canonical_command,
            None,
            &["out", "5"],
            Some(profile),
        );
        assert!(
            changed
                .conditional_source_nonempty_list_append(&input)
                .is_none()
        );
        for other in [
            None,
            Some(
                crate::model::ingress::resolve_known_environment("tcl8.6")
                    .unwrap()
                    .unit_profile(),
            ),
        ] {
            let input = LiteralInputs::new(selected.canonical_command, None, &["out", "5"], other);
            assert!(
                selected
                    .conditional_source_nonempty_list_append(&input)
                    .is_none()
            );
        }
    }
    #[test]
    fn list_append_requires_raw_and_effective_stock_representation() {
        for version in TclVersion::ALL {
            let profile =
                crate::model::ingress::resolve_known_environment(version.dialect_profile_name())
                    .unwrap()
                    .unit_profile();
            assert!(
                selected_relation(
                    version.dialect_profile_name(),
                    &["out", "5"],
                    SchemaChange::None
                )
                .is_some()
            );
            let baseline = crate::RepresentationEffect::copy_on_write_container(0, 2);
            let extra = crate::RepresentationEffect::CoerceOrdinaryList { operand: 1 };
            for override_at in 0..3 {
                let mut registry = CommandRegistry::build_default().project_for_profile(profile);
                let mut spec = registry.get("lappend").unwrap().clone();
                assert_eq!(spec.representation_effect, Some(baseline));
                if override_at != 1 {
                    spec.representation_effect = Some(extra);
                }
                registry.insert(spec);
                let args = ["out", "5"];
                let selected_words = [InvocationWord::Literal("out"), InvocationWord::Literal("5")];
                let mut selected = registry
                    .resolve_structured_invocation(
                        InvocationWords::from_arguments(
                            InvocationWord::Literal("lappend"),
                            InvocationArguments::structured(&selected_words)
                                .with_dialect(InvocationDialect::of_profile(profile)),
                        ),
                        Some(profile.surface_query()),
                    )
                    .resolved()
                    .unwrap();
                assert!(matches!(selected.semantics.value,
                    ResolvedSemantics::Derived(DerivedSemantics::CellUpdate(update))
                    if update.update == CellUpdate::ListAppend));
                match override_at {
                    0 => {
                        assert_eq!(selected.selected.spec.representation_effect, Some(extra));
                        assert_eq!(selected.semantics.representation_effect, extra);
                    }
                    1 => {
                        assert_eq!(selected.selected.spec.representation_effect, Some(baseline));
                        selected.semantics.representation_effect = extra;
                    }
                    _ => {
                        assert_eq!(selected.selected.spec.representation_effect, Some(extra));
                        selected.semantics.representation_effect = baseline;
                    }
                }
                let input =
                    LiteralInputs::new(selected.canonical_command, None, &args, Some(profile));
                assert!(
                    selected
                        .conditional_source_nonempty_list_append(&input)
                        .is_none(),
                    "{version:?}: representation override {override_at}"
                );
            }
        }
    }
    #[test]
    fn list_append_refuses_raw_world_and_state_descriptors_hidden_by_effective_view() {
        fn transitions(_: InvocationArguments<'_>) -> crate::StateTransitions {
            crate::StateTransitions::unknown_invocation()
        }
        for version in TclVersion::ALL {
            let profile =
                crate::model::ingress::resolve_known_environment(version.dialect_profile_name())
                    .unwrap()
                    .unit_profile();
            assert!(
                selected_relation(
                    version.dialect_profile_name(),
                    &["out", "5"],
                    SchemaChange::None
                )
                .is_some()
            );
            for state_descriptor in [false, true] {
                let mut registry = CommandRegistry::build_default().project_for_profile(profile);
                let mut spec = registry.get("lappend").unwrap().clone();
                if state_descriptor {
                    spec.state_transitions = Some(crate::StateTransitionDescriptor {
                        resolver: Some(transitions),
                        ..crate::StateTransitionDescriptor::EMPTY
                    });
                } else {
                    spec.world_effects = Some(crate::WorldEffectDescriptor::VARIABLE_WRITE);
                }
                registry.insert(spec);
                let args = ["out", "5"];
                let words = [InvocationWord::Literal("out"), InvocationWord::Literal("5")];
                let mut selected = registry
                    .resolve_structured_invocation(
                        InvocationWords::from_arguments(
                            InvocationWord::Literal("lappend"),
                            InvocationArguments::structured(&words)
                                .with_dialect(InvocationDialect::of_profile(profile)),
                        ),
                        Some(profile.surface_query()),
                    )
                    .resolved()
                    .unwrap();
                if state_descriptor {
                    assert!(selected.selected.spec.state_transitions.is_some());
                    assert!(selected.semantics.state_transitions.is_declared());
                    selected.semantics.state_transitions =
                        crate::state_transition::ResolvedStateTransitions::default();
                } else {
                    assert!(selected.selected.spec.world_effects.is_some());
                    assert!(selected.semantics.world_effects.is_declared());
                    selected.semantics.world_effects =
                        crate::world_effect::ResolvedWorldEffects::default();
                }
                assert!(!selected.semantics.state_transitions.is_declared());
                assert!(!selected.semantics.world_effects.is_declared());
                assert!(matches!(selected.semantics.value,
                    ResolvedSemantics::Derived(DerivedSemantics::CellUpdate(update))
                    if update.update == CellUpdate::ListAppend));
                let input =
                    LiteralInputs::new(selected.canonical_command, None, &args, Some(profile));
                assert!(
                    selected
                        .conditional_source_nonempty_list_append(&input)
                        .is_none(),
                    "{version:?}: raw descriptor hidden by public effective view"
                );
            }
        }
    }
    #[test]
    fn list_append_direct_phase_refuses_raw_and_effective_extra_execution_traits() {
        let profile = crate::model::ingress::resolve_known_environment("tcl9.0")
            .unwrap()
            .unit_profile();
        let args = ["out", "5"];
        let baseline = selected_relation("tcl9.0", &args, SchemaChange::None)
            .expect("active direct-phase control");
        assert!(
            baseline
                .conditional_direct_phase()
                .conditionally_preserves_lookup()
        );
        for extra in [
            crate::Traits::PERFORMS_SUBSTITUTION,
            crate::Traits::EVALUATES_CODE,
            crate::Traits::DEFERS_BODY,
            crate::Traits::CREATES_SCOPE_ALIAS,
            crate::Traits::ALIASES_GLOBAL,
            crate::Traits::ALIASES_CALLER_FRAME,
            crate::Traits::CURRENT_FRAME_INTROSPECTION,
            crate::Traits::EVALUATES_IN_SHIFTED_FRAME,
            crate::Traits::REPLACES_FRAME,
            crate::Traits::DYNAMIC_EVAL_BODY,
            crate::Traits::CREATES_BARRIER,
            crate::Traits::CREATES_DYNAMIC_BARRIER,
            crate::Traits::INVOKES_USER_PROC,
            crate::Traits::ESTABLISHES_VARIABLE_TRACE,
            crate::Traits::SOURCES_FILE,
            crate::Traits::DEFINES_PROCEDURE,
            crate::Traits::DECLARES_NAMESPACE,
        ] {
            for override_at in 0..3 {
                let mut registry = CommandRegistry::build_default().project_for_profile(profile);
                let mut spec = registry.get("lappend").unwrap().clone();
                let original_traits = spec.traits;
                assert!(!original_traits.intersects(extra));
                if override_at != 1 {
                    spec.traits |= extra;
                }
                registry.insert(spec);
                let mut selected = registry
                    .resolve_structured_invocation(
                        InvocationWords::from_arguments(
                            InvocationWord::Literal("lappend"),
                            InvocationArguments::literals(&args)
                                .with_dialect(InvocationDialect::of_profile(profile)),
                        ),
                        Some(profile.surface_query()),
                    )
                    .resolved()
                    .unwrap();
                match override_at {
                    0 => {
                        assert!(selected.selected.spec.traits.intersects(extra));
                        assert!(selected.semantics.traits.intersects(extra));
                    }
                    1 => {
                        assert_eq!(selected.selected.spec.traits, original_traits);
                        selected.semantics.traits |= extra;
                    }
                    _ => {
                        assert!(selected.selected.spec.traits.intersects(extra));
                        selected.semantics.traits = original_traits;
                    }
                }
                let input =
                    LiteralInputs::new(selected.canonical_command, None, &args, Some(profile));
                assert!(
                    selected
                        .conditional_source_nonempty_list_append(&input)
                        .is_none(),
                    "{extra:?}: override {override_at}"
                );
            }
        }
    }
    #[test]
    fn list_append_direct_phase_rejects_raw_frame_or_body_hidden_by_effective_view() {
        let profile = crate::model::ingress::resolve_known_environment("tcl9.0")
            .unwrap()
            .unit_profile();
        let args = ["out", "5"];
        for frame in [true, false] {
            let mut registry = CommandRegistry::build_default().project_for_profile(profile);
            let mut spec = registry.get("lappend").unwrap().clone();
            if frame {
                spec.frame_effect = Some(crate::frame_effect::FrameEffectSpec::UPVAR);
            } else {
                spec.body_execution =
                    Some(crate::body_execution::BodyExecutionSpec::CallerFrameSequence);
            }
            registry.insert(spec);
            let mut selected = registry
                .resolve_structured_invocation(
                    InvocationWords::from_arguments(
                        InvocationWord::Literal("lappend"),
                        InvocationArguments::literals(&args)
                            .with_dialect(InvocationDialect::of_profile(profile)),
                    ),
                    Some(profile.surface_query()),
                )
                .resolved()
                .unwrap();
            if frame {
                assert!(selected.selected.spec.frame_effect.is_some());
                selected.semantics.frame_effect = None;
            } else {
                assert!(selected.selected.spec.body_execution.is_some());
                selected.semantics.body_execution = None;
            }
            let input = LiteralInputs::new(selected.canonical_command, None, &args, Some(profile));
            assert!(
                selected
                    .conditional_source_nonempty_list_append(&input)
                    .is_none(),
                "raw frame={frame} hidden by public effective view"
            );
        }
    }
    #[test]
    fn list_append_direct_phase_requires_original_raw_native_and_handler_contract() {
        #[derive(Debug, Clone, Copy)]
        enum Mutation {
            MissingNative,
            Grammar,
            Operation,
            Body,
            MissingHandler,
            Handler,
        }
        let profile = crate::model::ingress::resolve_known_environment("tcl9.0")
            .unwrap()
            .unit_profile();
        let args = ["out", "5"];
        assert!(selected_relation("tcl9.0", &args, SchemaChange::None).is_some());
        for mutation in [
            Mutation::MissingNative,
            Mutation::Grammar,
            Mutation::Operation,
            Mutation::Body,
            Mutation::MissingHandler,
            Mutation::Handler,
        ] {
            let mut registry = CommandRegistry::build_default().project_for_profile(profile);
            let mut spec = registry.get("lappend").unwrap().clone();
            let stock_native = spec.native_compilation.unwrap();
            let stock_handler = spec.successful_handler;
            match mutation {
                Mutation::MissingNative => spec.native_compilation = None,
                Mutation::Grammar => {
                    spec.native_compilation.as_mut().unwrap().grammar =
                        NativeCompilationGrammar::Expression;
                }
                Mutation::Operation => {
                    spec.native_compilation.as_mut().unwrap().operation =
                        SemanticOperationId::Invoke;
                }
                Mutation::Body => {
                    spec.native_compilation.as_mut().unwrap().body = NativeBodyCompilation::Direct;
                }
                Mutation::MissingHandler => spec.successful_handler = None,
                Mutation::Handler => spec.successful_handler = Some(SuccessfulHandlerSpec::Leaf),
            }
            let raw_native = spec.native_compilation;
            let raw_handler = spec.successful_handler;
            registry.insert(spec);
            let mut selected = registry
                .resolve_structured_invocation(
                    InvocationWords::from_arguments(
                        InvocationWord::Literal("lappend"),
                        InvocationArguments::literals(&args)
                            .with_dialect(InvocationDialect::of_profile(profile)),
                    ),
                    Some(profile.surface_query()),
                )
                .resolved()
                .unwrap();
            assert_eq!(
                selected.selected.spec.native_compilation, raw_native,
                "{mutation:?}"
            );
            assert_eq!(
                selected.selected.spec.successful_handler, raw_handler,
                "{mutation:?}"
            );
            let input = LiteralInputs::new(selected.canonical_command, None, &args, Some(profile));
            assert!(
                selected
                    .conditional_source_nonempty_list_append(&input)
                    .is_none(),
                "{mutation:?}"
            );
            selected.semantics.native_compilation = Some(stock_native);
            selected.semantics.successful_handler = stock_handler;
            assert!(
                selected
                    .conditional_source_nonempty_list_append(&input)
                    .is_none(),
                "{mutation:?}: altered raw execution hidden by effective stock contract"
            );
        }
    }

    #[test]
    fn list_append_shared_stock_traits_keep_the_original_generic_bits() {
        let original = crate::Traits::FRAMELESS_RUNTIME
            | crate::Traits::NOT_PROC_FACTORY
            | crate::Traits::BYTE_COMPILED
            | crate::Traits::READS_BEFORE_WRITE
            | crate::Traits::FIRST_ARG_VARNAME
            | crate::Traits::UNCONDITIONAL_VARIABLE_WRITE;
        assert_eq!(CONDITIONAL_LIST_APPEND_TRAITS, original);
        for version in TclVersion::ALL {
            let profile =
                crate::model::ingress::resolve_known_environment(version.dialect_profile_name())
                    .unwrap()
                    .unit_profile();
            let registry = CommandRegistry::build_default().project_for_profile(profile);
            assert_eq!(
                registry.get("lappend").unwrap().traits,
                original,
                "{version:?}"
            );
        }
    }
}
