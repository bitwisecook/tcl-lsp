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

//! Successful paired-list iteration entry, independent of compiler admission.

use crate::native_compilation::NativeCompilationGrammar;
use crate::{InvocationArguments, InvocationFacts, SemanticOperationId};

/// Whether validated native list inputs require entry into an iteration body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IterationEntry {
    /// All iterator lists are empty; no body runs.
    Empty,
    /// At least one iterator list is nonempty after valid variable-list parsing.
    Required,
    /// A known cardinality or list operand is invalid; no body runs.
    Invalid,
    /// The selected protocol or an input needed for entry remains unknown.
    Unknown,
}

/// Original frozen paired-list inputs for a finite native iterator. Values
/// describe bytes only; object representation and store effects are independent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FiniteIteration {
    groups: Vec<FiniteIterationGroup>,
    iterations: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FiniteIterationGroup {
    names: Vec<Vec<u8>>,
    values: Vec<Vec<u8>>,
}

impl FiniteIteration {
    /// Number of actual iterations, including padding of shorter groups.
    #[must_use]
    pub fn iterations(&self) -> usize {
        self.iterations
    }

    /// Native ordered stores for this iteration. Missing elements supply the
    /// empty value; an out-of-range iteration supplies no store plan.
    pub fn bindings(&self, iteration: usize) -> Option<Vec<(&[u8], &[u8])>> {
        if iteration >= self.iterations {
            return None;
        }
        let mut bindings = Vec::new();
        for group in &self.groups {
            let start = iteration.checked_mul(group.names.len())?;
            for (index, name) in group.names.iter().enumerate() {
                let value = group
                    .values
                    .get(start.checked_add(index)?)
                    .map_or(&[][..], Vec::as_slice);
                bindings.push((name.as_slice(), value));
            }
        }
        Some(bindings)
    }
}

/// Conditional cardinality of the selected paired-list source schema.
///
/// This assumes the original handler executes successfully with the supplied
/// original literal values and ordinary list parsing. It grants no Native
/// argv, reached body, completed stores, object identity or execution proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConditionalSourceFiniteIteration {
    iterations: usize,
}

impl ConditionalSourceFiniteIteration {
    /// Number of iterations under the retained source premises.
    #[must_use]
    pub const fn iterations(self) -> usize {
        self.iterations
    }
}

/// One original literal loop-variable-list element under a conditional
/// successful C Tcl foreach handler. This is not an entered or completed store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConditionalSourceSingleLoopBinding {
    operand: usize,
    name: Vec<u8>,
    receiver: crate::resolved_invocation::VariableReceiverOperandForm,
    direct_phase_lookup_unchanged: bool,
}

impl ConditionalSourceSingleLoopBinding {
    /// Whether the audited direct phase leaves lookup unchanged under the
    /// ordinary-list and unobserved-cell premises. The separately modelled
    /// body retains its own effects and lookup obligations.
    #[must_use]
    pub const fn direct_phase_preserves_lookup(&self) -> bool {
        self.direct_phase_lookup_unchanged
    }

    /// Original varlist operand in the selected source schema.
    #[must_use]
    pub const fn operand(&self) -> usize {
        self.operand
    }

    /// The one decoded list element, not an actual runtime name value.
    #[must_use]
    pub fn name(&self) -> &[u8] {
        &self.name
    }

    /// Conditional handler receiver for that single decoded element.
    #[must_use]
    pub const fn receiver_form(&self) -> crate::resolved_invocation::VariableReceiverOperandForm {
        self.receiver
    }
}

/// Audited aggregate legacy effect of the stock paired-list loop. The
/// conditional source phase treats its body separately; generic facts retain it.
pub(crate) const PAIRED_LOOP_LEGACY_EFFECTS: &[crate::side_effects::SideEffect] =
    &[crate::side_effects::SideEffect {
        reads: true,
        writes: true,
        ..crate::side_effects::SideEffect::DEFAULT
    }];

/// Registry-owned opt-in for the stock ordinary-object paired-list handler.
/// This carries no entered body, store, callback or execution permission.
#[derive(Debug, Clone, Copy)]
pub struct ConditionalSourcePairedLoopHandler {
    _private: (),
}

/// Empty stock Jim handler phase under original authored-source premises.
/// This describes no variable store or body evaluation and grants no Native
/// argv, object class, physical frame, compilation or handler completion.
#[derive(Debug, Clone, Copy)]
pub struct AuthoredSourceJimEmptyIteration {
    policy: tcl_syntax::naming::NamePolicyProtocol,
}

impl AuthoredSourceJimEmptyIteration {
    /// Exact authored Jim naming recipe; execution authority is independent.
    #[must_use]
    pub const fn policy(self) -> tcl_syntax::naming::NamePolicyProtocol {
        self.policy
    }
}

pub(crate) const CONDITIONAL_SOURCE_PAIRED_LOOP_HANDLER: ConditionalSourcePairedLoopHandler =
    ConditionalSourcePairedLoopHandler { _private: () };

impl crate::ResolvedInvocation<'_, '_> {
    /// No binder or script is selected by the stock Jim handler after valid,
    /// empty original list operands. The caller must retain the original
    /// source words, completed argument effects and independent observer guards.
    #[must_use]
    pub fn authored_source_jim_empty_iteration(&self) -> Option<AuthoredSourceJimEmptyIteration> {
        use crate::hooks::LoweringHookId;
        use crate::native_compilation::{NativeBodyCompilation, SuccessfulHandlerSpec};
        use tcl_syntax::naming::{NamePolicyAuthority, NativeNameProtocol};

        let arguments = self.words.arguments();
        let dialect = arguments.dialect()?;
        let policy = dialect.authored_name_policy()?;
        let semantics = &self.semantics;
        semantics
            .value
            .semantics()?
            .conditional_source_paired_loop_handler()?;
        let native = semantics.native_compilation?;
        let facts = self.facts();
        if dialect.family() != Some(tcl_dialect::model::Family::Jim)
            || policy.authority() != NamePolicyAuthority::AuthoredSimulation
            || policy.recipe() != NativeNameProtocol::Jim084
            || self.selected.sub.is_some()
            || self.selected.instance
            || self.selected.form.is_some()
            || semantics.argument_offset != 0
            || !facts.arg_roles_complete
            || facts.operation != SemanticOperationId::StructuredLowering(LoweringHookId::Foreach)
            || native.grammar != NativeCompilationGrammar::Foreach
            || native.operation != facts.operation
            || native.body != NativeBodyCompilation::Inherit
            || semantics.successful_handler != Some(SuccessfulHandlerSpec::PossibleBodies)
            || semantics.body_kind != crate::body_kind::BodyKind::Plain
            || semantics.body_interpreter != crate::body_kind::BodyInterpreter::Current
            || semantics.frame_effect.is_some()
            || semantics.body_execution.is_some()
            || semantics.state_transitions.is_declared()
            || semantics.world_effects.is_declared()
            || semantics.side_effects != PAIRED_LOOP_LEGACY_EFFECTS
            || self.selected.spec.side_effects != PAIRED_LOOP_LEGACY_EFFECTS
            || facts.arity_accepts_frozen_arguments() != Some(true)
            || facts.iteration_entry(arguments) != IterationEntry::Empty
            || facts.finite_iteration(arguments)?.iterations() != 0
        {
            return None;
        }
        Some(AuthoredSourceJimEmptyIteration { policy })
    }

    /// One source binder under the selected original handler, ordinary list,
    /// unobserved cell and separately modelled body premises.
    #[must_use]
    pub fn conditional_source_single_loop_binding(
        &self,
    ) -> Option<ConditionalSourceSingleLoopBinding> {
        use crate::hooks::LoweringHookId;
        use crate::native_compilation::{
            NativeBodyCompilation, NativeCompilationGrammar, SuccessfulHandlerSpec,
        };
        let arguments = self.words.arguments();
        let dialect = arguments.dialect()?;
        let semantics = &self.semantics;
        semantics
            .value
            .semantics()?
            .conditional_source_paired_loop_handler()?;
        let native = semantics.native_compilation?;
        if self.selected.spec.side_effects != PAIRED_LOOP_LEGACY_EFFECTS
            || self.selected.sub.is_some()
            || self.selected.instance
            || self.selected.form.is_some()
            || dialect.family() != Some(tcl_dialect::model::Family::Tcl)
            || dialect.tcl_version.is_none()
            || self.facts().operation
                != SemanticOperationId::StructuredLowering(LoweringHookId::Foreach)
            || native.grammar != NativeCompilationGrammar::Foreach
            || native.operation != SemanticOperationId::StructuredLowering(LoweringHookId::Foreach)
            || native.body != NativeBodyCompilation::Inherit
            || semantics.successful_handler != Some(SuccessfulHandlerSpec::PossibleBodies)
            || semantics.body_kind != crate::body_kind::BodyKind::Plain
            || semantics.body_interpreter != crate::body_kind::BodyInterpreter::Current
            || semantics.frame_effect.is_some()
            || semantics.body_execution.is_some()
            || self.facts().arity_accepts_frozen_arguments() != Some(true)
            || semantics.state_transitions.is_declared()
            || semantics.world_effects.is_declared()
            || semantics.side_effects != PAIRED_LOOP_LEGACY_EFFECTS
        {
            return None;
        }
        let shape = self.facts().finite_iteration_shape(arguments)?;
        let [group] = shape.groups.as_slice() else {
            return None;
        };
        let [name] = group.names.as_slice() else {
            return None;
        };
        Some(ConditionalSourceSingleLoopBinding {
            operand: semantics.argument_offset,
            name: name.clone(),
            receiver: crate::resolved_invocation::VariableReceiverOperandForm::Combined,
            direct_phase_lookup_unchanged: true,
        })
    }
}

impl InvocationFacts {
    /// Exact native paired-list rows from complete frozen literal operands.
    /// This uses the same selected contract as iteration entry, retaining
    /// native list parsing independently from word spelling and compiler hooks.
    #[must_use]
    pub fn finite_iteration(&self, arguments: InvocationArguments<'_>) -> Option<FiniteIteration> {
        self.finite_iteration_shape(arguments)
    }

    /// Paired-list cardinality only under the original-source premises.
    /// This conditional view cannot be converted to native iteration proof.
    #[must_use]
    pub fn conditional_source_finite_iteration(
        &self,
        arguments: InvocationArguments<'_>,
    ) -> Option<ConditionalSourceFiniteIteration> {
        self.finite_iteration_shape(arguments)
            .map(|shape| ConditionalSourceFiniteIteration {
                iterations: shape.iterations,
            })
    }

    fn finite_iteration_shape(
        &self,
        arguments: InvocationArguments<'_>,
    ) -> Option<FiniteIteration> {
        if !matches!(
            self.iteration_entry(arguments),
            IterationEntry::Empty | IterationEntry::Required
        ) {
            return None;
        }
        let dialect = arguments.dialect()?;
        let mut groups = Vec::new();
        let mut iterations = 0;
        let decode = |index| {
            tcl_syntax::list::split_list_bytes_in(
                arguments.literal_at(index)?.as_bytes(),
                dialect.word_values.list,
                dialect.lexer_grammar.escapes,
            )
            .ok()
            .map(|items| {
                items
                    .into_iter()
                    .map(std::borrow::Cow::into_owned)
                    .collect::<Vec<_>>()
            })
        };
        for variable in (self.argument_offset..arguments.len().checked_sub(1)?).step_by(2) {
            let names = decode(variable)?;
            let values = decode(variable + 1)?;
            if names.is_empty() {
                return None;
            }
            iterations = iterations.max(values.len().div_ceil(names.len()));
            groups.push(FiniteIterationGroup { names, values });
        }
        Some(FiniteIteration { groups, iterations })
    }

    /// Native paired-list entry proof from actual frozen operands. This proves
    /// no handler identity, variable-store success, observer freedom or opcode.
    #[must_use]
    pub fn iteration_entry(&self, arguments: InvocationArguments<'_>) -> IterationEntry {
        use crate::hooks::LoweringHookId;
        use IterationEntry::{Empty, Invalid, Required, Unknown};
        if !matches!(self.native_compilation, Some(contract)
            if contract.grammar == NativeCompilationGrammar::Foreach)
            || !matches!(
                self.operation,
                SemanticOperationId::StructuredLowering(
                    LoweringHookId::Foreach | LoweringHookId::Lmap
                )
            )
        {
            return Unknown;
        }
        let Some(dialect) = arguments.dialect() else {
            return Unknown;
        };
        let Some(count) = arguments.exact_argv_len() else {
            return Unknown;
        };
        let Some(count) = count.checked_sub(self.argument_offset) else {
            return Invalid;
        };
        if count < 3 || count % 2 == 0 {
            return Invalid;
        }
        if count + self.argument_offset != arguments.len() {
            return Unknown;
        }
        let mut values_unknown = false;
        let mut required = false;
        let mut variables_unknown = false;
        for variable in (self.argument_offset..arguments.len() - 1).step_by(2) {
            let list_length = |index| {
                arguments.literal_at(index).map(|text| {
                    tcl_syntax::list::split_list_bytes_in(
                        text.as_bytes(),
                        dialect.word_values.list,
                        dialect.lexer_grammar.escapes,
                    )
                    .map(|elements| elements.len())
                })
            };
            match list_length(variable) {
                Some(Ok(0) | Err(_)) => return Invalid,
                Some(Ok(_)) => {}
                None => variables_unknown = true,
            }
            match list_length(variable + 1) {
                Some(Err(_)) => return Invalid,
                Some(Ok(length)) => required |= length > 0,
                None => values_unknown = true,
            }
        }
        if variables_unknown {
            Unknown
        } else if required {
            Required
        } else if values_unknown {
            Unknown
        } else {
            Empty
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{InvocationWord, InvocationWords};

    #[test]
    fn finite_native_pairs_keep_order_padding_and_dynamic_withdrawals() {
        use InvocationWord::{Dynamic, Literal as L};
        for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let profile = crate::model::ingress::resolve_environment(environment).unit_profile();
            let registry = crate::CommandRegistry::build_default();
            let words = [L("x y"), L("a b c"), L("z"), L("1 2 3"), L("")];
            let arguments = InvocationArguments::structured(&words)
                .with_dialect(crate::InvocationDialect::of_profile(profile));
            let selection = registry.resolve_structured_invocation(
                InvocationWords::from_arguments(L("foreach"), arguments),
                None,
            );
            let invocation = selection.resolved().unwrap();
            let facts = invocation.facts();
            let finite = facts.finite_iteration(arguments).unwrap();
            assert_eq!(finite.iterations(), 3);
            assert_eq!(
                finite.bindings(0).unwrap(),
                [
                    (b"x".as_slice(), b"a".as_slice()),
                    (b"y".as_slice(), b"b".as_slice()),
                    (b"z".as_slice(), b"1".as_slice())
                ]
            );
            assert_eq!(
                finite.bindings(1).unwrap(),
                [
                    (b"x".as_slice(), b"c".as_slice()),
                    (b"y".as_slice(), b"".as_slice()),
                    (b"z".as_slice(), b"2".as_slice())
                ]
            );
            assert_eq!(
                finite.bindings(2).unwrap(),
                [
                    (b"x".as_slice(), b"".as_slice()),
                    (b"y".as_slice(), b"".as_slice()),
                    (b"z".as_slice(), b"3".as_slice())
                ]
            );
            assert!(finite.bindings(3).is_none());
            let words = [L("x"), Dynamic, L("")];
            let unknown = InvocationArguments::structured(&words)
                .with_dialect(crate::InvocationDialect::of_profile(profile));
            assert!(facts.finite_iteration(unknown).is_none());
        }
    }

    #[test]
    fn conditional_source_count_retains_only_known_paired_list_shape() {
        use InvocationWord::{Dynamic, Expanded, Literal as L};
        for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let profile = crate::model::ingress::resolve_environment(environment).unit_profile();
            let dialect = crate::InvocationDialect::of_profile(profile);
            let registry = crate::CommandRegistry::build_default();
            let seed = [L("i"), L("1 2 3"), L("")];
            let arguments = InvocationArguments::structured(&seed).with_dialect(dialect);
            let selection = registry.resolve_structured_invocation(
                InvocationWords::from_arguments(L("foreach"), arguments),
                None,
            );
            let selected = selection.resolved().unwrap();
            let facts = selected.facts();
            let binding = selected.conditional_source_single_loop_binding();
            if environment == "jim" {
                assert!(binding.is_none());
            } else {
                let binding = binding.expect("conditional single binder");
                assert_eq!(binding.operand(), 0);
                assert_eq!(binding.name(), b"i");
                assert!(binding.direct_phase_preserves_lookup());
                assert_eq!(
                    binding.receiver_form(),
                    crate::resolved_invocation::VariableReceiverOperandForm::Combined
                );
            }
            let multiple = [L("i j"), L("1 2"), L("")];
            let multiple = InvocationArguments::structured(&multiple).with_dialect(dialect);
            let multiple_selection = registry.resolve_structured_invocation(
                InvocationWords::from_arguments(L("foreach"), multiple),
                None,
            );
            assert!(
                multiple_selection
                    .resolved()
                    .unwrap()
                    .conditional_source_single_loop_binding()
                    .is_none()
            );
            for (values, expected) in [("", 0), ("1", 1), ("1 2 3", 3)] {
                let words = [L("i"), L(values), L("")];
                let args = InvocationArguments::structured(&words).with_dialect(dialect);
                assert_eq!(
                    facts
                        .conditional_source_finite_iteration(args)
                        .unwrap()
                        .iterations(),
                    expected,
                    "{environment} {values:?}"
                );
                assert_eq!(facts.finite_iteration(args).unwrap().iterations(), expected);
            }
            for words in [
                [L("i"), Dynamic, L("")],
                [Expanded, L("1"), L("")],
                [L(""), L("1"), L("")],
            ] {
                let args = InvocationArguments::structured(&words).with_dialect(dialect);
                assert!(
                    facts.conditional_source_finite_iteration(args).is_none(),
                    "{environment} {words:?}"
                );
            }
            let words = [L("i"), L("{"), L("")];
            let args = InvocationArguments::structured(&words).with_dialect(dialect);
            let count = facts
                .conditional_source_finite_iteration(args)
                .map(ConditionalSourceFiniteIteration::iterations);
            assert_eq!(count, (environment == "jim").then_some(1), "{environment}");
        }
    }

    #[test]
    fn conditional_single_binder_declines_other_profiles_and_handlers() {
        use InvocationWord::Literal as L;
        let registry = crate::CommandRegistry::build_default();
        let profile = crate::model::ingress::resolve_environment("tcl9.0").unit_profile();
        let dialect = crate::InvocationDialect::of_profile(profile);
        let words = [L("i"), L("1 2 3"), L("")];
        let arguments = InvocationArguments::structured(&words).with_dialect(dialect);
        let lmap = registry.resolve_structured_invocation(
            InvocationWords::from_arguments(L("lmap"), arguments),
            None,
        );
        assert!(
            lmap.resolved()
                .unwrap()
                .conditional_source_single_loop_binding()
                .is_none()
        );
        let foreach = registry.resolve_structured_invocation(
            InvocationWords::from_arguments(L("foreach"), arguments),
            None,
        );
        let selected = foreach.resolved().unwrap();
        assert!(selected.conditional_source_single_loop_binding().is_some());
        let mut extra = selected;
        const EXTRA: &[crate::side_effects::SideEffect] = &[
            crate::iteration_entry::PAIRED_LOOP_LEGACY_EFFECTS[0],
            crate::side_effects::SideEffect {
                target: crate::side_effects::SideEffectTarget::FileIo,
                writes: true,
                ..crate::side_effects::SideEffect::DEFAULT
            },
        ];
        extra.semantics.side_effects = EXTRA;
        assert!(extra.conditional_source_single_loop_binding().is_none());
        let mut declined = selected;
        declined.semantics.value =
            crate::value_transfer::declaration::ResolvedSemantics::Declined {
                scope: crate::value_transfer::declaration::DeclarationScope::Form,
            };
        assert!(declined.conditional_source_single_loop_binding().is_none());
        let mut explicit_world = selected;
        explicit_world.semantics.world_effects.form =
            Some(crate::world_effect::WorldEffectDescriptor::EMPTY);
        assert!(
            explicit_world
                .conditional_source_single_loop_binding()
                .is_none()
        );
        let mut transition = selected;
        transition.semantics.state_transitions.form =
            Some(crate::state_transition::StateTransitionDescriptor::EMPTY);
        assert!(
            transition
                .conditional_source_single_loop_binding()
                .is_none()
        );
        let mut invalid_arity = selected;
        invalid_arity.semantics.arity = crate::Arity::exact(0);
        assert!(
            invalid_arity
                .conditional_source_single_loop_binding()
                .is_none()
        );
        let mut extra_body = selected;
        extra_body.semantics.body_execution =
            Some(crate::body_execution::BodyExecutionSpec::DeferredGlobalScript);
        assert!(
            extra_body
                .conditional_source_single_loop_binding()
                .is_none()
        );
        let mut wrong_body = selected;
        wrong_body.semantics.successful_handler =
            Some(crate::native_compilation::SuccessfulHandlerSpec::Leaf);
        assert!(
            wrong_body
                .conditional_source_single_loop_binding()
                .is_none()
        );
        let effects = selected.facts().effects;
        assert_eq!(
            effects.callback().kinds,
            crate::world_effect::CallbackKinds::UNKNOWN
        );
        assert!(
            effects
                .accesses()
                .iter()
                .any(|access| access.domain
                    == crate::world_effect::WorldStateDomain::HostCapabilities)
        );
        let f5 = crate::model::ingress::resolve_environment("f5-irules").unit_profile();
        let arguments = InvocationArguments::structured(&words)
            .with_dialect(crate::InvocationDialect::of_profile(f5));
        let f5 = registry.resolve_structured_invocation(
            InvocationWords::from_arguments(L("foreach"), arguments),
            None,
        );
        assert!(
            f5.resolved()
                .unwrap()
                .conditional_source_single_loop_binding()
                .is_none()
        );
    }

    #[test]
    fn conditional_paired_loop_refuses_effects_hidden_by_form_override() {
        use crate::side_effects::{SideEffect, SideEffectTarget};
        use crate::{ArgRole, Arity};
        use InvocationWord::Literal as L;
        const EXTRA: &[SideEffect] = &[
            PAIRED_LOOP_LEGACY_EFFECTS[0],
            SideEffect {
                target: SideEffectTarget::FileIo,
                writes: true,
                ..SideEffect::DEFAULT
            },
        ];
        const FORMS: &[crate::forms::CommandForm] = &[crate::forms::CommandForm {
            name: "three",
            arity: Arity::exact(3),
            arg_roles: &[(0, ArgRole::LoopVarList), (2, ArgRole::Body)],
            side_effects: Some(PAIRED_LOOP_LEGACY_EFFECTS),
            ..crate::forms::CommandForm::DEFAULT
        }];
        let mut registry = crate::CommandRegistry::build_default();
        let mut spec = registry.get("foreach").unwrap().clone();
        spec.name = "test-paired-loop";
        spec.side_effects = EXTRA;
        spec.command_forms = FORMS;
        registry.insert(spec);
        let profile = crate::model::ingress::resolve_environment("tcl9.0").unit_profile();
        let words = [L("i"), L("1 2 3"), L("")];
        let arguments = InvocationArguments::structured(&words)
            .with_dialect(crate::InvocationDialect::of_profile(profile));
        let resolution = registry.resolve_structured_invocation(
            InvocationWords::from_arguments(L("test-paired-loop"), arguments),
            None,
        );
        let selected = resolution.resolved().expect("selected fixture");
        assert!(selected.form.is_some());
        assert_eq!(selected.semantics.side_effects, PAIRED_LOOP_LEGACY_EFFECTS);
        assert_eq!(selected.selected.spec.side_effects, EXTRA);
        assert!(selected.conditional_source_single_loop_binding().is_none());
    }

    fn entry(environment: &str, command: &str, values: &[InvocationWord<'_>]) -> IterationEntry {
        let profile = crate::model::ingress::resolve_environment(environment).unit_profile();
        let dialect = crate::InvocationDialect::of_profile(profile);
        let registry = crate::CommandRegistry::build_default();
        let arguments = InvocationArguments::structured(values).with_dialect(dialect);
        let Some(invocation) = registry
            .resolve_structured_invocation(
                InvocationWords::from_arguments(InvocationWord::Literal(command), arguments),
                None,
            )
            .resolved()
        else {
            return IterationEntry::Unknown;
        };
        invocation.facts().iteration_entry(arguments)
    }

    #[test]
    fn native_pairs_distinguish_required_empty_invalid_and_unknown_entry() {
        use InvocationWord::{Dynamic, Expanded, Literal as L};
        use IterationEntry::{Empty, Invalid, Required, Unknown};
        for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            assert_eq!(
                entry(environment, "foreach", &[L("x"), L("a"), L("")]),
                Required
            );
            assert_eq!(
                entry(environment, "foreach", &[L("x"), L(""), L("")]),
                Empty
            );
            assert_eq!(
                entry(environment, "foreach", &[L("x"), Dynamic, L("")]),
                Unknown
            );
            assert_eq!(
                entry(environment, "foreach", &[Dynamic, L("a"), L("")]),
                Unknown
            );
            assert_eq!(
                entry(environment, "foreach", &[L(""), L("a"), L("")]),
                Invalid
            );
            assert_eq!(
                entry(
                    environment,
                    "foreach",
                    &[L("x"), L("a"), L("y"), Dynamic, L("")]
                ),
                Required
            );
            assert_eq!(
                entry(environment, "foreach", &[Expanded, L("a"), L("")]),
                Unknown
            );
            let malformed = entry(environment, "foreach", &[L("x"), L("{"), L("")]);
            assert_eq!(
                malformed,
                if environment == "jim" {
                    Required
                } else {
                    Invalid
                }
            );
        }
        assert_eq!(entry("tcl8.6", "lmap", &[L("x"), L("a"), L("")]), Required);
        assert_eq!(entry("tcl8.4", "lmap", &[L("x"), L("a"), L("")]), Unknown);
        assert_eq!(
            entry("tcl9.1", "array", &[L("for"), L("x"), L("a"), L("")]),
            Unknown
        );
    }
}
