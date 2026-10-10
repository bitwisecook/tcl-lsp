// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Completed original-source values, independently of total completion.

use super::{
    Arc, CommandAllocationSite, DeferredImplementationId, DeferredSourceBody, SourceBindingPoint,
    SourceCommandBindings, SourceCommandTarget, SourceInvocationBinding, SourceOutcomes,
    native_result::EvaluatedSourceValue,
};
use std::collections::BTreeMap;

/// The value on all represented Normal alternatives of an original evaluation.
/// This supplies neither a physical object nor an absence-of-error proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceNormalResult {
    value: Arc<EvaluatedSourceValue>,
}

impl SourceNormalResult {
    /// Original completed result bytes, exposed by the source text owner.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.value.text
    }

    /// Independently joined representation; equal bytes do not imply a class.
    #[must_use]
    pub fn representation(&self) -> tcl_syntax::value::ValueRepresentation {
        self.value.representation
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct NormalOutcome {
    normal_possible: bool,
    result: Option<SourceNormalResult>,
    complete_normally: bool,
}

impl NormalOutcome {
    fn of(outcomes: &SourceOutcomes) -> Self {
        Self {
            normal_possible: outcomes.normal.is_some(),
            result: outcomes
                .normal
                .as_ref()
                .and(outcomes.normal_value.as_ref())
                .map(|value| SourceNormalResult {
                    value: Arc::clone(value),
                }),
            complete_normally: outcomes.normal.is_some()
                && outcomes.normal_completion.is_some()
                && outcomes.abrupt.is_empty(),
        }
    }

    fn join(&mut self, incoming: &Self) {
        if incoming.normal_possible {
            self.result = if self.normal_possible {
                self.result
                    .as_ref()
                    .zip(incoming.result.as_ref())
                    .and_then(|(left, right)| {
                        super::native_result::joined_result_values(&left.value, &right.value)
                            .map(|value| SourceNormalResult { value })
                    })
            } else {
                incoming.result.clone()
            };
        }
        self.normal_possible |= incoming.normal_possible;
        self.complete_normally &= incoming.complete_normally;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct InvocationResultBasis {
    binding: SourceInvocationBinding,
    words: Vec<crate::ir::WordExpr>,
}

impl InvocationResultBasis {
    pub(super) fn of(point: &SourceBindingPoint, words: &[crate::ir::WordExpr]) -> Self {
        Self {
            binding: SourceCommandBindings::query_dispatch_points(std::iter::once(point)),
            words: words.to_vec(),
        }
    }

    fn matches(
        &self,
        binding: &SourceInvocationBinding,
        tokens: &crate::ir::CommandTokens,
    ) -> bool {
        let original = &self.binding;
        tokens.synthetic.is_none()
            && self.words == tokens.words()
            && original.dispatch_site == binding.dispatch_site
            && original.lookup_namespace == binding.lookup_namespace
            && original.lookup_word == binding.lookup_word
            && original.variable_frame == binding.variable_frame
            && original.variable_context == binding.variable_context
            && original.targets == binding.targets
            && original.may_be_absent == binding.may_be_absent
            && original.unknown == binding.unknown
            && original.compiled_candidates == binding.compiled_candidates
            && original.compiled_named_candidates == binding.compiled_named_candidates
            && original.compiled_execution_residual == binding.compiled_execution_residual
            && original.native_compilation_admission == binding.native_compilation_admission
            && original.native_operand_layout == binding.native_operand_layout
            && original.evaluated_argument_values == binding.evaluated_argument_values
            && original.evaluated_argument_words == binding.evaluated_argument_words
            && original.frozen_written_words == binding.frozen_written_words
            && original.frozen_written_names == binding.frozen_written_names
            && original.frozen_head_object == binding.frozen_head_object
            && original.runtime_reachability == binding.runtime_reachability
            && original.entered_execution_observer == binding.entered_execution_observer
            && binding.dispatch_site.as_ref().is_some_and(|site| {
                crate::registry_invocation::native_compiler_replay_source(tokens, site).is_some()
            })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct InvocationResultObservation {
    basis: InvocationResultBasis,
    outcome: NormalOutcome,
}

pub(super) type InvocationNormalResults =
    BTreeMap<CommandAllocationSite, Vec<InvocationResultObservation>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ConditionalProcedureResult {
    declaration: DeferredSourceBody,
    outcome: NormalOutcome,
}

pub(super) type ConditionalProcedureResults =
    BTreeMap<DeferredImplementationId, Vec<ConditionalProcedureResult>>;

/// Equivalence under an exact original scoped procedure-entry interpretation.
/// This proves neither document entry nor an arbitrary future interpreter world.
pub(crate) struct SourceScopedProcedureEvaluation<'a> {
    pub(crate) target: &'a SourceCommandTarget,
    pub(crate) result: &'a SourceNormalResult,
}

impl SourceInvocationBinding {
    /// Exact scoped normal result plus independent complete-evaluation proof.
    /// All captured world, compiler, source, argv and declaration owners remain
    /// prerequisites. This receipt cannot supply actual dispatch or native CPP.
    pub(crate) fn scoped_procedure_evaluation(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<SourceScopedProcedureEvaluation<'_>> {
        let target = self.scoped_unobserved_target(tokens)?;
        if target.registry_backed
            || target.kind != super::BindingKind::Proc
            || target
                .implementation_allocation
                .as_ref()
                .is_none_or(|allocation| {
                    allocation.incarnation == super::AllocationIncarnation::RepeatedFresh
                })
        {
            return None;
        }
        let observations = self.normal_result_observations.as_deref()?;
        let first = observations.first()?;
        let result = first.outcome.result.as_ref()?;
        observations
            .iter()
            .all(|observation| {
                observation.basis.matches(self, tokens)
                    && observation.outcome.complete_normally
                    && observation.outcome.result.as_ref() == Some(result)
            })
            .then_some(SourceScopedProcedureEvaluation { target, result })
    }

    pub(super) fn scoped_unobserved_target(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<&SourceCommandTarget> {
        if self.runtime_reachability != super::SourceRuntimeReachability::Conditional {
            return None;
        }
        let site = self.invocation_site()?;
        crate::registry_invocation::native_compiler_replay_source(tokens, site)?;
        let declarations = super::declaration_layout::original_declaration_layouts(
            self.declaration_layout_observations.as_deref()?,
        )?;
        let first = declarations.clone().next()?;
        let super::declaration_layout::OriginalDiagnosticFrameEntry::Body(entry) =
            first.entry.as_ref()
        else {
            return None;
        };
        if entry.namespace_context().is_none()
            || !entry.owns_invocation(self)
            || declarations.clone().any(|observation| {
                observation.entry != first.entry
                    || !observation.entry.owns_source(&site.source, site.offset)
            })
        {
            return None;
        }
        let state = &self.lookup_state.as_ref()?.state;
        if state.has_opaque_domain()
            || state.source_step_observed()
            || state.source_execution_observed(None)
            || self.entered_execution_observer.observed()
        {
            return None;
        }
        self.closed_selected_execution_target()
    }

    /// Completed result at the same original dispatch and bound argv. Unknown
    /// normal routes withdraw bytes; Error alternatives do not imply totality.
    #[must_use]
    pub fn original_normal_result(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<&SourceNormalResult> {
        if self.runtime_reachability == super::SourceRuntimeReachability::Conditional {
            return None;
        }
        let observations = self.normal_result_observations.as_deref()?;
        let first = observations.first()?;
        if observations
            .iter()
            .any(|observation| !observation.basis.matches(self, tokens))
        {
            return None;
        }
        let result = first.outcome.result.as_ref()?;
        observations
            .iter()
            .all(|observation| observation.outcome.result.as_ref() == Some(result))
            .then_some(result)
    }

    /// Independent proof that every represented original evaluation settles
    /// normally. A known conditional result alone cannot establish this.
    #[must_use]
    pub fn original_invocation_completes_normally(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> bool {
        if self.runtime_reachability == super::SourceRuntimeReachability::Conditional {
            return false;
        }
        self.normal_result_observations
            .as_deref()
            .is_some_and(|observations| {
                !observations.is_empty()
                    && observations.iter().all(|observation| {
                        observation.basis.matches(self, tokens)
                            && observation.outcome.complete_normally
                    })
            })
    }
}

impl SourceCommandBindings {
    pub(super) fn record_invocation_normal_result(
        &mut self,
        basis: Box<InvocationResultBasis>,
        outcomes: &SourceOutcomes,
    ) {
        let Some(site) = basis.binding.dispatch_site.clone() else {
            return;
        };
        let outcome = NormalOutcome::of(outcomes);
        let observations = self.invocation_normal_results.entry(site).or_default();
        if let Some(retained) = observations
            .iter_mut()
            .find(|observation| observation.basis == *basis)
        {
            retained.outcome.join(&outcome);
        } else {
            observations.push(InvocationResultObservation {
                basis: *basis,
                outcome,
            });
        }
    }

    pub(super) fn attach_invocation_normal_result(
        &self,
        binding: &mut SourceInvocationBinding,
        origin: Option<&Arc<super::SourceOriginId>>,
        offset: u32,
    ) {
        binding.normal_result_observations = origin.and_then(|origin| {
            self.invocation_normal_results
                .get(&CommandAllocationSite {
                    source: Arc::clone(origin),
                    offset,
                })
                .map(|observations| Arc::from(observations.as_slice()))
        });
    }

    pub(super) fn record_conditional_procedure_result(
        &mut self,
        declaration: &DeferredSourceBody,
        outcomes: &SourceOutcomes,
        dialect: Option<tcl_registry::InvocationDialect>,
    ) {
        if declaration.receiver_method
            || declaration.event.is_some()
            || declaration.future_frame.is_some()
            || declaration.statics.is_some()
        {
            return;
        }
        let settled = outcomes.through_procedure_boundary(dialect);
        let outcome = NormalOutcome::of(&settled);
        let observations = self
            .conditional_procedure_results
            .entry(declaration.implementation_id())
            .or_default();
        if let Some(retained) = observations
            .iter_mut()
            .find(|observation| observation.declaration == *declaration)
        {
            retained.outcome.join(&outcome);
        } else {
            observations.push(ConditionalProcedureResult {
                declaration: declaration.clone(),
                outcome,
            });
        }
    }

    pub(super) fn merge_conditional_procedure_results(&mut self, other: &Self) {
        for (implementation, observations) in &other.conditional_procedure_results {
            let retained = self
                .conditional_procedure_results
                .entry(implementation.clone())
                .or_default();
            for observation in observations {
                if let Some(existing) = retained
                    .iter_mut()
                    .find(|existing| existing.declaration == observation.declaration)
                {
                    existing.outcome.join(&observation.outcome);
                } else {
                    retained.push(observation.clone());
                }
            }
        }
    }

    /// Result of the original declaration body conditional on its own entry,
    /// with unknown formal inputs. Actual caller constants cannot supply it.
    /// Selection, formal binding, effects and totality remain separate proofs.
    #[must_use]
    pub fn conditional_procedure_normal_result(
        &self,
        target: &SourceCommandTarget,
    ) -> Option<&SourceNormalResult> {
        self.procedure_implementation_body(target)?;
        let declaration = self.deferred.get(&target.implementation_id())?;
        let observations = self
            .conditional_procedure_results
            .get(&target.implementation_id())?;
        let first = observations.first()?;
        let result = first.outcome.result.as_ref()?;
        observations
            .iter()
            .all(|observation| {
                observation.declaration == *declaration
                    && observation.outcome.result.as_ref() == Some(result)
            })
            .then_some(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn value(
        text: &str,
        representation: tcl_syntax::value::ValueRepresentation,
    ) -> SourceNormalResult {
        SourceNormalResult {
            value: Arc::new(EvaluatedSourceValue {
                text: text.to_owned(),
                representation,
                numeric: None,
            }),
        }
    }

    #[test]
    fn normal_result_unknown_routes_poison_bytes_without_donating_totality() {
        use tcl_syntax::value::ValueRepresentation::Unknown;
        let mut known = NormalOutcome {
            normal_possible: true,
            result: Some(value("3", Unknown)),
            complete_normally: true,
        };
        let missing = NormalOutcome {
            normal_possible: true,
            result: None,
            complete_normally: false,
        };
        known.join(&missing);
        known.join(&NormalOutcome {
            normal_possible: true,
            result: Some(value("3", Unknown)),
            complete_normally: true,
        });
        assert!(known.result.is_none());
        assert!(!known.complete_normally);
        let mut partial = NormalOutcome {
            normal_possible: true,
            result: Some(value("3", Unknown)),
            complete_normally: true,
        };
        partial.join(&NormalOutcome {
            normal_possible: false,
            result: None,
            complete_normally: false,
        });
        assert_eq!(partial.result.as_ref().unwrap().text(), "3");
        assert!(!partial.complete_normally);
    }

    #[test]
    fn equal_normal_bytes_join_representation_independently() {
        use tcl_syntax::value::ValueRepresentation;
        let mut result = NormalOutcome {
            normal_possible: true,
            result: Some(value("", ValueRepresentation::List)),
            complete_normally: false,
        };
        result.join(&NormalOutcome {
            normal_possible: true,
            result: Some(value("", ValueRepresentation::Dict)),
            complete_normally: false,
        });
        assert_eq!(result.result.as_ref().unwrap().text(), "");
        assert_eq!(
            result.result.as_ref().unwrap().representation(),
            ValueRepresentation::Unknown
        );
        result.join(&NormalOutcome {
            normal_possible: true,
            result: Some(value("different", ValueRepresentation::Unknown)),
            complete_normally: false,
        });
        assert!(result.result.is_none());
    }

    fn inventory(source: &str) -> (SourceCommandBindings, tcl_lexer::LexerConfig) {
        inventory_for(source, "tcl8.6")
    }

    fn inventory_for(
        source: &str,
        profile_name: &str,
    ) -> (SourceCommandBindings, tcl_lexer::LexerConfig) {
        let owner = tcl_registry::model::ingress::static_context_for(profile_name);
        let registry = owner.commands();
        let profile = registry.profile().unwrap();
        let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
        let bindings = SourceCommandBindings::analyse_with_options(
            source,
            config,
            registry,
            super::super::SourceAnalysisOptions {
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                native_compilation: crate::environment_ingress::authoring_native_compilation(),
                ..Default::default()
            },
        );
        (bindings, config)
    }

    fn original_tokens(
        source: &str,
        offset: usize,
        config: tcl_lexer::LexerConfig,
    ) -> crate::ir::CommandTokens {
        let commands = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
        let command = commands
            .iter()
            .find(|command| usize::try_from(command.span.start()).unwrap() == offset)
            .unwrap();
        crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::new(source),
            config,
            command,
        )
    }

    #[test]
    fn original_closed_leaf_completion_keeps_definition_and_store_obligations_separate() {
        // Implementation contract: naming.source.closed-leaf-handler-completion
        // docs/design/analysis/name-resolution-proofs/closed-leaf-handler-completion.md
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let source = "proc deferred {} {error LATER}";
            let (bindings, config) = inventory_for(source, profile);
            let tokens = original_tokens(source, 0, config);
            let binding = bindings.invocation_at_source("proc", 0);
            assert!(
                binding.original_invocation_completes_normally(&tokens),
                "{profile}"
            );
            assert!(
                bindings.original_completed_root_state.is_some(),
                "{profile}"
            );

            let source = "set result [set target VALUE]";
            let (bindings, config) = inventory_for(source, profile);
            let tokens = original_tokens(source, 0, config);
            assert!(
                bindings
                    .invocation_at_source("set", 0)
                    .original_invocation_completes_normally(&tokens),
                "{profile}"
            );
            assert!(
                bindings.original_completed_root_state.is_some(),
                "{profile}"
            );

            for source in [
                "proc P {{bad extra fields}} {}",
                "set a(k) 1; set a 2",
                "set result [error ARGUMENT]",
            ] {
                let (bindings, _) = inventory_for(source, profile);
                assert!(
                    bindings.original_completed_root_state.is_none(),
                    "{profile}: {source}"
                );
            }
        }
    }

    #[test]
    fn original_jim_fresh_store_completion_uses_the_selected_write_kernel() {
        // Implementation contract: naming.source.closed-leaf-handler-completion
        // docs/design/analysis/name-resolution-proofs/closed-leaf-handler-completion.md
        let dialect = tcl_registry::InvocationDialect::of_point(
            tcl_dialect::model::DialectPoint::of_dialect_name(Some("jim")).unwrap(),
        );
        let registry = tcl_registry::model::ingress::static_context_for("jim").commands();
        let config = tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar);
        let analyse = |source| {
            SourceCommandBindings::analyse_with_options(
                source,
                config,
                registry,
                super::super::SourceAnalysisOptions {
                    invocation_dialect: Some(dialect),
                    native_compilation:
                        tcl_registry::native_compilation::NativeCompilationContext {
                            mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
                            ..Default::default()
                        },
                    ..Default::default()
                },
            )
        };
        let source = "set result [set target VALUE]";
        let bindings = analyse(source);
        let tokens = original_tokens(source, 0, config);
        assert!(
            bindings
                .invocation_at_source("set", 0)
                .original_invocation_completes_normally(&tokens)
        );
        assert!(bindings.original_completed_root_state.is_some());
        let unknown = analyse("set result $missing");
        assert!(unknown.original_completed_root_state.is_none());
    }

    #[test]
    fn scoped_procedure_equivalence_keeps_independent_world_and_completion_obligations() {
        let make = |source: &str| {
            let (bindings, config) = inventory_for(source, "tcl9.0");
            let offset = source.find("[answer]").unwrap() + 1;
            let segment = crate::segmenter::segment_commands_with_offset_and_config(
                "answer",
                u32::try_from(offset).unwrap(),
                config,
            )
            .remove(0);
            let mut tokens = crate::ir::CommandTokens::from_segmented(
                &tcl_lexer::SourceMap::new(source),
                config,
                &segment,
            );
            bindings.stamp_original_tokens(&mut tokens);
            tokens
        };
        let positive = make("proc answer {} {return 42}; proc f {} {return [answer]}");
        let binding = positive.source_binding.as_ref().unwrap();
        assert!(binding.proved_execution_target().is_none());
        assert!(!binding.original_invocation_completes_normally(&positive));
        let proof = binding
            .scoped_procedure_evaluation(&positive)
            .expect("independent original scoped certificates");
        assert_eq!(proof.target.command, "::answer");
        assert_eq!(proof.result.text(), "42");
        for source in [
            "proc answer {} {return 42}; proc f {} {return [answer]}; unknown_future_entry",
            "proc answer {} {return 42}; proc f {} {return [answer]}; trace add execution answer enter callback",
            "proc answer {} {unknown_callback; return 42}; proc f {} {return [answer]}",
        ] {
            let tokens = make(source);
            assert!(
                tokens
                    .source_binding
                    .as_ref()
                    .unwrap()
                    .scoped_procedure_evaluation(&tokens)
                    .is_none(),
                "{source}"
            );
        }
        let mut foreign = positive.clone();
        foreign.synthetic = Some(crate::ir::SyntheticMarker::IterationBindings(None));
        assert!(binding.scoped_procedure_evaluation(&foreign).is_none());
    }

    #[test]
    fn original_normal_results_validate_source_words_and_relocation() {
        let source = "set x 3; set y 4";
        let (bindings, config) = inventory(source);
        let tokens = original_tokens(source, 0, config);
        let binding = bindings.invocation_at_source("set", 0);
        assert_eq!(binding.original_normal_result(&tokens).unwrap().text(), "3");
        let foreign = original_tokens("set x 9; set y 4", 0, config);
        assert!(binding.original_normal_result(&foreign).is_none());
        let other = bindings.invocation_at_source("set", 9);
        assert!(other.original_normal_result(&tokens).is_none());
        let mut discarded = binding;
        discarded.relocate_variable_proofs(&crate::var_resolve::VariableProofRelocation::default());
        assert!(discarded.original_normal_result(&tokens).is_none());
    }

    #[test]
    fn caller_specific_normal_results_do_not_become_declaration_constants() {
        let source = "proc f {x} {set local $x}; f 3; f 4";
        let (bindings, config) = inventory(source);
        for (needle, expected) in [("f 3", "3"), ("f 4", "4")] {
            let offset = source.find(needle).unwrap();
            let tokens = original_tokens(source, offset, config);
            let binding = bindings.invocation_at_source("f", u32::try_from(offset).unwrap());
            assert_eq!(
                binding.original_normal_result(&tokens).unwrap().text(),
                expected
            );
            assert!(
                bindings
                    .conditional_procedure_normal_result(binding.proved_handler_target().unwrap())
                    .is_none()
            );
        }
    }

    #[test]
    fn native_integer_call_results_and_mutable_double_spelling_are_independent() {
        for (profile, fixture, precision_one) in [
            (
                "tcl8.4",
                include_str!("../../tests/data/native_integer_contents_results/tcl8.4.txt"),
                "2e+01",
            ),
            (
                "tcl8.5",
                include_str!("../../tests/data/native_integer_contents_results/tcl8.5.txt"),
                "20.0",
            ),
            (
                "tcl8.6",
                include_str!("../../tests/data/native_integer_contents_results/tcl8.6.txt"),
                "20.0",
            ),
            (
                "tcl9.0",
                include_str!("../../tests/data/native_integer_contents_results/tcl9.0.txt"),
                "21.0",
            ),
            (
                "tcl9.1",
                include_str!("../../tests/data/native_integer_contents_results/tcl9.1.txt"),
                "21.0",
            ),
            (
                "jim",
                include_str!("../../tests/data/native_integer_contents_results/jim.txt"),
                "21.0",
            ),
        ] {
            assert_eq!(fixture.lines().count(), 10, "{profile}");
            for row in [
                "formal21 0 42",
                "formal22 0 44",
                "fixed_with_rest 0 10",
                "unrelated_rename 0 42",
                "known_double 0 2.0",
                "double_default 0 21.0",
            ] {
                assert!(fixture.lines().any(|line| line == row), "{profile}: {row}");
            }
            assert!(
                fixture
                    .lines()
                    .any(|line| line == format!("double_precision_one 0 {precision_one}")),
                "{profile}"
            );
            assert!(
                fixture
                    .lines()
                    .any(|line| line.starts_with("invalid_number 1 ")),
                "{profile}"
            );
            assert!(
                fixture
                    .lines()
                    .any(|line| line.starts_with("division_error 1 ")),
                "{profile}"
            );
        }
    }

    #[test]
    fn mutable_double_precision_does_not_publish_unproved_result_bytes() {
        let source = "proc f {} {expr {double(21)}}; f";
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6"] {
            let (bindings, config) = inventory_for(source, profile);
            let offset = source.rfind('f').unwrap();
            let tokens = original_tokens(source, offset, config);
            let binding = bindings.invocation_at_source("f", u32::try_from(offset).unwrap());
            assert!(
                binding.original_normal_result(&tokens).is_none(),
                "{profile}"
            );
        }
    }

    #[test]
    fn accepted_original_integer_formals_produce_call_specific_normal_results() {
        for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let source = "proc double {n} {expr {$n * 2}}; double 21; double 22";
            let (bindings, config) = inventory_for(source, profile);
            for (needle, expected) in [("double 21", "42"), ("double 22", "44")] {
                let offset = source.find(needle).unwrap();
                let tokens = original_tokens(source, offset, config);
                let binding =
                    bindings.invocation_at_source("double", u32::try_from(offset).unwrap());
                assert_eq!(
                    binding.original_normal_result(&tokens).unwrap().text(),
                    expected,
                    "{profile}"
                );
                assert!(
                    binding.original_invocation_completes_normally(&tokens),
                    "{profile}"
                );
                assert!(
                    bindings
                        .conditional_procedure_normal_result(
                            binding.proved_handler_target().unwrap()
                        )
                        .is_none()
                );
            }

            let source = "proc f {a args} {expr {$a * 2}}; f 5 x y z";
            let (bindings, config) = inventory_for(source, profile);
            let offset = source.find("f 5").unwrap();
            let tokens = original_tokens(source, offset, config);
            let binding = bindings.invocation_at_source("f", u32::try_from(offset).unwrap());
            assert_eq!(
                binding.original_normal_result(&tokens).unwrap().text(),
                "10",
                "{profile}"
            );
            assert!(
                binding.original_invocation_completes_normally(&tokens),
                "{profile}"
            );
        }
    }

    #[test]
    fn integer_formal_result_withdraws_unknown_values_observers_and_failures() {
        for source in [
            "proc f {n} {expr {$n * 2}}; f $unknown",
            "proc f {n} {unknown_host; expr {$n * 2}}; f 21",
            "proc f {n} {expr {$n / 0}}; f 21",
            "proc f {n} {expr {$n * 2}}; f not_a_number",
        ] {
            let (bindings, config) = inventory(source);
            let offset = source.rfind("; f ").unwrap() + 2;
            let tokens = original_tokens(source, offset, config);
            let binding = bindings.invocation_at_source("f", u32::try_from(offset).unwrap());
            assert!(
                binding.original_normal_result(&tokens).is_none(),
                "{source}"
            );
            assert!(
                !binding.original_invocation_completes_normally(&tokens),
                "{source}"
            );
        }
    }

    #[test]
    fn selected_double_completion_matches_six_native_engines() {
        for (profile, fixture) in [
            (
                "tcl8.4",
                include_str!("../../tests/data/native_double_completion/tcl8.4.txt"),
            ),
            (
                "tcl8.5",
                include_str!("../../tests/data/native_double_completion/tcl8.5.txt"),
            ),
            (
                "tcl8.6",
                include_str!("../../tests/data/native_double_completion/tcl8.6.txt"),
            ),
            (
                "tcl9.0",
                include_str!("../../tests/data/native_double_completion/tcl9.0.txt"),
            ),
            (
                "tcl9.1",
                include_str!("../../tests/data/native_double_completion/tcl9.1.txt"),
            ),
            (
                "jim",
                include_str!("../../tests/data/native_double_completion/jim.txt"),
            ),
        ] {
            assert_eq!(fixture.lines().count(), 10, "{profile}");
            for row in [
                "double21 0 21.0",
                "negative 0 -21.0",
                "argument_return 0 EARLY",
                "argument_error 1 ARGUMENT",
            ] {
                assert!(fixture.lines().any(|line| line == row), "{profile}: {row}");
            }
            for id in ["invalid", "arity_zero", "arity_two"] {
                assert!(
                    fixture
                        .lines()
                        .any(|line| line.starts_with(&format!("{id} 1 "))),
                    "{profile}: {id}"
                );
            }
            let replacement = if matches!(profile, "tcl8.4" | "jim") {
                "replacement 0 21.0"
            } else {
                "replacement 1 REPLACED"
            };
            assert!(fixture.lines().any(|line| line == replacement), "{profile}");
            assert!(
                fixture.lines().any(|line| line
                    == if profile == "jim" {
                        "read_trace_unavailable 0 unavailable"
                    } else {
                        "read_error 1 {can't read \"original\": READ_ERROR}"
                    }),
                "{profile}"
            );
        }
    }

    #[test]
    fn called_double_formals_do_not_seed_declaration_results() {
        let source = "proc f {n} {expr {double($n)}}; f 21; f 22";
        let (bindings, config) = inventory_for(source, "tcl9.0");
        for (needle, expected) in [("f 21", "21.0"), ("f 22", "22.0")] {
            let offset = source.find(needle).unwrap();
            let tokens = original_tokens(source, offset, config);
            let binding = bindings.invocation_at_source("f", u32::try_from(offset).unwrap());
            assert_eq!(
                binding.original_normal_result(&tokens).unwrap().text(),
                expected
            );
            assert!(binding.original_invocation_completes_normally(&tokens));
            assert!(
                bindings
                    .conditional_procedure_normal_result(binding.proved_handler_target().unwrap())
                    .is_none()
            );
        }
    }

    #[test]
    fn called_declaration_result_analysis_withdraws_namespace_input_contents() {
        let source = "set n 21; proc f {} {expr {double($::n)}}; f";
        let (bindings, config) = inventory_for(source, "tcl9.0");
        let offset = source.rfind('f').unwrap();
        let tokens = original_tokens(source, offset, config);
        let binding = bindings.invocation_at_source("f", u32::try_from(offset).unwrap());
        assert_eq!(
            binding.original_normal_result(&tokens).unwrap().text(),
            "21.0"
        );
        assert!(
            bindings
                .conditional_procedure_normal_result(binding.proved_handler_target().unwrap())
                .is_none()
        );
    }

    #[test]
    fn uncalled_double_declaration_keeps_its_independent_normal_result() {
        let source = "proc f {} {expr {double(21)}}";
        let (bindings, _) = inventory_for(source, "tcl9.0");
        let binding = super::super::source_binding(&bindings.final_state, "f", "::");
        assert_eq!(
            bindings
                .conditional_procedure_normal_result(binding.proved_handler_target().unwrap())
                .unwrap()
                .text(),
            "21.0"
        );
    }

    #[test]
    fn double_domain_errors_do_not_acquire_normal_completion() {
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            for source in [
                "expr {double(\"invalid\")}",
                "expr {double()}",
                "expr {double(1,2)}",
            ] {
                let (bindings, config) = inventory_for(source, profile);
                let tokens = original_tokens(source, 0, config);
                let binding = bindings.invocation_at_source("expr", 0);
                assert!(
                    binding.original_normal_result(&tokens).is_none(),
                    "{profile}: {source}"
                );
                assert!(
                    !binding.original_invocation_completes_normally(&tokens),
                    "{profile}: {source}"
                );
            }
        }
    }

    #[test]
    fn double_read_callbacks_withdraw_normal_completion() {
        let source = "proc read_error args {error READ_ERROR}; set n 21; trace add variable n read read_error; expr {double($n)}";
        let (bindings, config) = inventory_for(source, "tcl9.0");
        let offset = source.rfind("expr").unwrap();
        let tokens = original_tokens(source, offset, config);
        let binding = bindings.invocation_at_source("expr", u32::try_from(offset).unwrap());
        assert!(binding.original_normal_result(&tokens).is_none());
        assert!(!binding.original_invocation_completes_normally(&tokens));
    }

    #[test]
    fn selected_numeric_call_results_keep_completion_and_rebinding_separate() {
        let source = "proc f {} {expr {double(21)}}; f";
        // Tcl 9's native double string policy is immutable. Tcl 8 precision
        // is thread-wide state and is not supplied by this source receipt.
        let (bindings, config) = inventory_for(source, "tcl9.0");
        let offset = source.rfind('f').unwrap();
        let tokens = original_tokens(source, offset, config);
        let binding = bindings.invocation_at_source("f", u32::try_from(offset).unwrap());
        assert_eq!(
            binding.original_normal_result(&tokens).unwrap().text(),
            "21.0"
        );
        assert!(binding.original_invocation_completes_normally(&tokens));
        assert_eq!(
            bindings
                .conditional_procedure_normal_result(binding.proved_handler_target().unwrap())
                .unwrap()
                .text(),
            "21.0"
        );

        let source =
            "proc ::tcl::mathfunc::double args {error CUSTOM}; proc f {} {expr {double(21)}}; f";
        let (bindings, config) = inventory_for(source, "tcl9.0");
        let offset = source.rfind('f').unwrap();
        let tokens = original_tokens(source, offset, config);
        let binding = bindings.invocation_at_source("f", u32::try_from(offset).unwrap());
        assert!(binding.original_normal_result(&tokens).is_none());
        assert!(!binding.original_invocation_completes_normally(&tokens));
    }

    #[test]
    fn default_return_totality_requires_the_original_procedure_boundary() {
        let source = "proc f {} {return 42}; f";
        let (bindings, config) = inventory(source);
        let offset = source.rfind('f').unwrap();
        let tokens = original_tokens(source, offset, config);
        let binding = bindings.invocation_at_source("f", u32::try_from(offset).unwrap());
        assert_eq!(
            binding.original_normal_result(&tokens).unwrap().text(),
            "42"
        );
        assert!(binding.original_invocation_completes_normally(&tokens));

        for source in [
            "proc f {} {return -level 2 42}; f",
            "proc f {} {return -code error 42}; f",
            "proc f {} {notacommand; return 42}; f",
        ] {
            let (bindings, config) = inventory(source);
            let offset = source.rfind('f').unwrap();
            let tokens = original_tokens(source, offset, config);
            let binding = bindings.invocation_at_source("f", u32::try_from(offset).unwrap());
            assert!(
                !binding.original_invocation_completes_normally(&tokens),
                "{source}"
            );
        }
    }

    #[test]
    fn declaration_normal_result_meets_all_return_and_fallthrough_routes() {
        let source = "proc f {flag} {if {$flag} {return 3}; set z 4}; f 1";
        let (bindings, _) = inventory(source);
        let binding = bindings
            .invocation_at_source("f", u32::try_from(source.rfind("f 1").unwrap()).unwrap());
        assert!(
            bindings
                .conditional_procedure_normal_result(binding.proved_handler_target().unwrap())
                .is_none()
        );
    }
}
