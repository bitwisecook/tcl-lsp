// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Conditional source operation coverage, independently of successful execution.

use crate::side_effects::SideEffect;
use crate::{
    EffectFootprint, InvocationDialect, StateTransitions, TransitionEffectCoverage,
    TransitionEffectCoverages,
};

/// Registry-authored coverage of a primitive's conditional result transitions.
/// Only the registered source resolver can construct this metadata.
#[derive(Debug, Clone)]
pub struct SourceResultTransitionCoverage {
    pub(crate) transitions: StateTransitions,
    pub(crate) coverage: TransitionEffectCoverages,
    pub(crate) legacy_effects: &'static [SideEffect],
}
impl SourceResultTransitionCoverage {
    pub(crate) fn new(
        transitions: StateTransitions,
        coverage: &'static [TransitionEffectCoverage],
        legacy_effects: &'static [SideEffect],
    ) -> Self {
        let mut selected = TransitionEffectCoverages::default();
        selected.extend(coverage);
        Self {
            transitions,
            coverage: selected,
            legacy_effects,
        }
    }
}

/// Independent runtime premises retained by every conditional source operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceOperationObligation {
    /// The original Registry implementation is selected at this source point.
    OriginalHandler,
    /// Existing dispatch, name-lookup and execution observers remain applicable.
    ObserverApplicability,
    /// Conditional result transitions do not prove that execution reached OK.
    ResultCompletion,
}

/// One complete original source operation and its result-qualified effect split.
/// The full footprint is retained; no mutation or observer debt is erased.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoredSourceOperationContinuation {
    pub(crate) descriptor: String,
    pub(crate) selectors: Vec<&'static str>,
    pub(crate) form: Option<&'static str>,
    pub(crate) dispatch_dependencies: crate::DispatchDependencies,
    pub(crate) words: Vec<String>,
    pub(crate) dialect: InvocationDialect,
    pub(crate) effects: EffectFootprint,
    pub(crate) residual: EffectFootprint,
    pub(crate) result_transitions: StateTransitions,
    pub(crate) coverage: TransitionEffectCoverages,
}
impl AuthoredSourceOperationContinuation {
    /// Exact selected Registry metadata label, without handler identity.
    #[must_use]
    pub fn descriptor(&self) -> &str {
        &self.descriptor
    }
    /// Actual selected subcommand path, without reconstructing its spelling.
    #[must_use]
    pub fn selectors(&self) -> &[&'static str] {
        &self.selectors
    }
    /// Actual selected form, independently of sibling form vocabulary.
    #[must_use]
    pub const fn form(&self) -> Option<&'static str> {
        self.form
    }
    /// Registry dispatch dependencies remain independent of result coverage.
    #[must_use]
    pub const fn dispatch_dependencies(&self) -> crate::DispatchDependencies {
        self.dispatch_dependencies
    }
    /// Complete source-value vector, including its original effective head.
    #[must_use]
    pub fn words(&self) -> &[String] {
        &self.words
    }
    /// Independently supplied source argument grammar.
    #[must_use]
    pub const fn dialect(&self) -> InvocationDialect {
        self.dialect
    }
    /// Full raw effects before any declaration-qualified result split.
    #[must_use]
    pub const fn effects(&self) -> &EffectFootprint {
        &self.effects
    }
    /// Noncovered synchronous dependencies; reads remain independent premises.
    #[must_use]
    pub const fn residual(&self) -> &EffectFootprint {
        &self.residual
    }
    /// Source result intentions, never a committed state or Normal outcome.
    #[must_use]
    pub const fn result_transitions(&self) -> &StateTransitions {
        &self.result_transitions
    }
    /// Exact declaration sources/domains represented by result transitions.
    #[must_use]
    pub const fn result_coverage(&self) -> &TransitionEffectCoverages {
        &self.coverage
    }
    /// Unmet premises remain explicit even when no synchronous body is present.
    #[must_use]
    pub const fn obligations(&self) -> &'static [SourceOperationObligation] {
        &[
            SourceOperationObligation::OriginalHandler,
            SourceOperationObligation::ObserverApplicability,
            SourceOperationObligation::ResultCompletion,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{InvocationWord, InvocationWords};
    fn select(arguments: &[InvocationWord<'_>]) -> Option<AuthoredSourceOperationContinuation> {
        let context = crate::model::ingress::static_context_for("tcl8.6");
        let dialect = crate::InvocationDialect::of_profile(context.commands().profile()?);
        crate::model::assembly::resolve_structured_invocation_in_resolved_context(
            context.commands(),
            Some(context.context()),
            InvocationWords::structured(InvocationWord::Literal("trace"), arguments)
                .with_dialect(dialect),
            tcl_dialect::model::InvocationRealm::RuleLoader,
        )
        .resolved()?
        .authored_source_operation_continuation(dialect)
    }
    #[test]
    fn source_result_coverage_retains_raw_writes_residual_reads_and_unmet_premises() {
        use InvocationWord::{Dynamic, Expanded, Literal};
        let operation = select(&[
            Literal("add"),
            Literal("variable"),
            Literal("other"),
            Literal("write"),
            Literal("callback"),
        ])
        .unwrap();
        assert!(
            operation
                .effects()
                .accesses()
                .iter()
                .any(|access| access.mode != crate::EffectAccessMode::Read)
        );
        assert!(
            operation
                .residual()
                .accesses()
                .iter()
                .all(|access| access.mode == crate::EffectAccessMode::Read)
        );
        assert_ne!(operation.residual().accesses().len(), 0);
        assert_eq!(operation.obligations().len(), 3);
        assert!(
            operation
                .obligations()
                .contains(&SourceOperationObligation::ObserverApplicability)
        );
        assert_eq!(
            operation.words(),
            ["trace", "add", "variable", "other", "write", "callback"]
        );
        for arguments in [
            vec![
                Literal("add"),
                Literal("variable"),
                Dynamic,
                Literal("write"),
                Literal("callback"),
            ],
            vec![
                Literal("add"),
                Literal("variable"),
                Literal("other"),
                Literal("bogus"),
                Literal("callback"),
            ],
            vec![
                Literal("add"),
                Literal("variable"),
                Literal("other"),
                Expanded,
                Literal("callback"),
            ],
            vec![
                Literal("add"),
                Literal("execution"),
                Literal("other"),
                Literal("enter"),
                Literal("callback"),
            ],
            vec![
                Literal("add"),
                Literal("variable"),
                Literal("other"),
                Literal("write"),
            ],
        ] {
            assert!(select(&arguments).is_none());
        }
    }

    #[test]
    fn source_coverage_never_drops_extra_effects_or_borrows_a_missing_hook() {
        use InvocationWord::Literal;
        let context = crate::model::ingress::static_context_for("tcl8.6");
        let dialect = crate::InvocationDialect::of_profile(context.commands().profile().unwrap());
        let arguments = [
            Literal("add"),
            Literal("variable"),
            Literal("other"),
            Literal("write"),
            Literal("callback"),
        ];
        let mut selected =
            crate::model::assembly::resolve_structured_invocation_in_resolved_context(
                context.commands(),
                Some(context.context()),
                InvocationWords::structured(Literal("trace"), &arguments).with_dialect(dialect),
                tcl_dialect::model::InvocationRealm::RuleLoader,
            )
            .resolved()
            .unwrap();
        assert!(
            selected
                .authored_source_operation_continuation(dialect)
                .is_some()
        );
        let original = selected.semantics.world_effects.subcommand.unwrap();
        selected
            .semantics
            .world_effects
            .subcommand
            .as_mut()
            .unwrap()
            .source_resolver = None;
        assert!(
            selected
                .authored_source_operation_continuation(dialect)
                .is_none()
        );
        selected.semantics.world_effects.subcommand = Some(crate::WorldEffectDescriptor {
            static_footprint: crate::StaticEffectFootprint {
                accesses: &[crate::StaticEffectAccess::VARIABLE_STORE_WILDCARD],
                callback: crate::CallbackEffect::NONE,
            },
            ..original
        });
        assert!(
            selected
                .authored_source_operation_continuation(dialect)
                .is_none(),
            "an extra static write is not covered by the resolver's result contract"
        );
        selected.semantics.world_effects.subcommand = Some(original);
        selected.semantics.state_transitions.subcommand =
            Some(crate::StateTransitionDescriptor::EMPTY);
        assert!(
            selected
                .authored_source_operation_continuation(dialect)
                .is_none(),
            "the actual selected transition chain must reproduce the audited result facts"
        );
    }
}
