// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Actual sole-expression reads paired with their later captured native stores.

use super::{
    Arc, CommandAllocationSite, ModuleCommandBindings, SourceCommandBindings,
    SourceExecutionContext, SourceExpressionPreparation, SourceInvocationBinding,
    SourceNativeInvocation, SourceOriginId, SourceVariableEvaluationOwner,
};
use crate::{ir::SourceSite, place::Place, var_resolve::ResolveContext};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct CapturedExpressionRead {
    pub(super) expression: CommandAllocationSite,
    setter: CommandAllocationSite,
    pub(super) operand: SourceSite,
    pub(super) place: Place,
    pub(super) before: Arc<ResolveContext>,
}

/// One actual read and its later original setter in the same evaluation.
/// This proves address/order only. Numeric acceptance, representation,
/// handlers and equivalence require their independent owner receipts.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceReadStoreObservation {
    read: Arc<CapturedExpressionRead>,
    setter_context: Arc<ResolveContext>,
    setter_place: Place,
}

impl SourceReadStoreObservation {
    /// Exact original expression invocation, including its retained source instance.
    #[must_use]
    pub fn expression(&self) -> &CommandAllocationSite {
        &self.read.expression
    }
    /// Original variable occurrence mapped by that expression preparation;
    /// equal offsets in another source do not identify this read.
    #[must_use]
    pub fn operand(&self) -> &SourceSite {
        &self.read.operand
    }
    /// Physical context before the original RHS read and runtime coercion.
    /// Later cache normalisation cannot establish this earlier category.
    #[must_use]
    pub fn read_context(&self) -> &ResolveContext {
        &self.read.before
    }
    /// Captured original scalar address with a known cell generation.
    /// Contents, numeric acceptance and observer proofs remain independent.
    #[must_use]
    pub fn read_place(&self) -> &Place {
        &self.read.place
    }
    /// Physical context at the actual selected setter after argv evaluation
    /// and handler preparation, before its captured write.
    #[must_use]
    pub fn setter_context(&self) -> &ResolveContext {
        &self.setter_context
    }
    /// Captured destination paired with this read in its own observation.
    /// A proposed replacement must resolve the same address in `setter_context`.
    #[must_use]
    pub fn setter_place(&self) -> &Place {
        &self.setter_place
    }
}

pub(super) type ReadStoreObservations =
    BTreeMap<CommandAllocationSite, Option<Vec<SourceReadStoreObservation>>>;

impl SourceInvocationBinding {
    /// Complete original read/store observations. Missing or conflicting
    /// evaluation coverage remains unknown, including after relocation.
    #[must_use]
    pub fn sole_rhs_read_store_observations(&self) -> Option<&[SourceReadStoreObservation]> {
        self.rhs_read_store_observations.as_deref()
    }
}

fn parent_argument_site<'a>(
    mut owner: Option<&'a SourceVariableEvaluationOwner>,
    expression: &CommandAllocationSite,
) -> Option<&'a CommandAllocationSite> {
    while let Some(current) = owner {
        match current {
            SourceVariableEvaluationOwner::InvocationArguments { invocation, parent } => {
                if invocation != expression {
                    return Some(invocation);
                }
                owner = parent.as_deref();
            }
            SourceVariableEvaluationOwner::NativeExpression { parent, .. }
            | SourceVariableEvaluationOwner::InvocationBody { parent, .. } => {
                owner = parent.as_deref();
            }
            SourceVariableEvaluationOwner::Alternatives(_)
            | SourceVariableEvaluationOwner::Unspecified => return None,
        }
    }
    None
}

impl CapturedExpressionRead {
    pub(super) fn capture(
        preparation: &SourceExpressionPreparation,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> Option<Arc<Self>> {
        let schedule = preparation.witness.increment_expression_schedule()?;
        let (_, operand) = preparation.original_variable_source(schedule.operand())?;
        let crate::expr_ast::ExprNode::Var { text, .. } = schedule.operand() else {
            return None;
        };
        let before = &state.source_variables;
        let place = crate::var_resolve::resolve_substitution_access(
            text,
            before,
            context.registry,
            tcl_registry::TraceOperation::Read,
        );
        if place.observed
            || place.dynamic
            || place.index.is_some()
            || place.kind != crate::place::PlaceKind::Scalar
            || place
                .cell
                .as_ref()
                .is_none_or(|cell| cell.generation == crate::place::CellGeneration::Unknown)
            || !before.read_produces_value(&place, context.registry)
            || !before.contents_native_string_access_closed_at(&place, context.registry)
            || state.ordinary_literal_pool.is_none()
            || state.has_opaque_domain()
            || state.source_step_observed()
        {
            return None;
        }
        Some(Arc::new(Self {
            expression: preparation.invocation.clone(),
            setter: parent_argument_site(context.variable_read_owner, &preparation.invocation)?
                .clone(),
            operand,
            place,
            before: Arc::clone(before),
        }))
    }

    pub(super) fn observed_original_read(
        &self,
        reads: &[super::SourceVariableAccess],
        registry: &tcl_registry::CommandRegistry,
    ) -> bool {
        reads.iter().any(|read| {
            read.source == self.operand
                && read.context_residual() == super::SourceVariableReadResidual::Closed
                && matches!(&read.owner,
                    SourceVariableEvaluationOwner::NativeExpression { invocation, .. }
                    if invocation == &self.expression)
                && read.context_alternatives().iter().any(|context| {
                    context == &self.before
                        && read.place_in_context(context, registry) == self.place
                })
        })
    }

    pub(super) fn remains_current(
        &self,
        state: &ModuleCommandBindings,
        registry: &tcl_registry::CommandRegistry,
    ) -> bool {
        state.current_source_origin.as_ref() == Some(&self.expression.source)
            && !state.has_opaque_domain()
            && state
                .source_variables
                .read_produces_value(&self.place, registry)
            && state.source_variables.contents_origin(&self.place)
                == self.before.contents_origin(&self.place)
            && state.source_variables.contents_source(&self.place)
                == self.before.contents_source(&self.place)
    }
}

impl SourceCommandBindings {
    pub(super) fn completed_expression_read(
        &self,
        read: Option<Arc<CapturedExpressionRead>>,
        outcomes: &super::SourceOutcomes,
        invocation_offset: u32,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<Arc<CapturedExpressionRead>> {
        read.filter(|read| {
            outcomes
                .normal
                .as_ref()
                .is_some_and(|normal| read.remains_current(normal, registry))
                && read.observed_original_read(
                    &self.variable_accesses_during_expression_invocation(invocation_offset),
                    registry,
                )
        })
    }

    pub(super) fn record_rhs_read_store(
        &mut self,
        native: SourceNativeInvocation<'_>,
        facts: &tcl_registry::InvocationFacts,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) {
        if facts.operation
            != tcl_registry::SemanticOperationId::StructuredLowering(
                tcl_registry::hooks::LoweringHookId::Set,
            )
        {
            return;
        }
        let Some(origin) = &state.current_source_origin else {
            return;
        };
        let site = CommandAllocationSite {
            source: Arc::clone(origin),
            offset: native.segment.span.start(),
        };
        let observation = capture_store(native, facts, state, context, &site);
        self.rhs_read_store_observations
            .entry(site)
            .and_modify(|previous| match (previous.as_mut(), observation.as_ref()) {
                (Some(previous), Some(observation)) => {
                    if !previous.contains(observation) {
                        previous.push(observation.clone());
                    }
                }
                _ => *previous = None,
            })
            .or_insert_with(|| observation.map(|observation| vec![observation]));
    }

    pub(super) fn attach_rhs_read_store(
        &self,
        binding: &mut SourceInvocationBinding,
        origin: Option<&Arc<SourceOriginId>>,
        offset: u32,
    ) {
        binding.rhs_read_store_observations = origin.and_then(|origin| {
            self.rhs_read_store_observations
                .get(&CommandAllocationSite {
                    source: Arc::clone(origin),
                    offset,
                })?
                .as_ref()
                .map(|observations| Arc::from(observations.as_slice()))
        });
    }
}

fn capture_store(
    native: SourceNativeInvocation<'_>,
    facts: &tcl_registry::InvocationFacts,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
    site: &CommandAllocationSite,
) -> Option<SourceReadStoreObservation> {
    if !native.target.prepended.is_empty()
        || native.words.len() != 3
        || native.invocation.arguments().exact_argv_len() != Some(2)
        || state.has_opaque_domain()
        || state.source_step_observed()
        || state.source_execution_observed(native.target.identity.as_ref())
    {
        return None;
    }
    let read = context
        .written_variable_reads?
        .get(2)?
        .as_ref()?
        .expression
        .as_ref()?;
    if &read.setter != site
        || !read.remains_current(state, context.registry)
        || native.words.get(2)?.sole_command_substitution().is_none()
    {
        return None;
    }
    let writes = crate::variable_bindings::source_variable_write_places(
        facts,
        native.invocation.arguments(),
        &state.source_variables,
        context.registry,
    );
    let [setter_place] = writes.as_slice() else {
        return None;
    };
    if setter_place != &read.place
        || setter_place.observed
        || setter_place.dynamic
        || state.source_variables.store_would_error(setter_place)
    {
        return None;
    }
    Some(SourceReadStoreObservation {
        read: Arc::clone(read),
        setter_context: Arc::clone(&state.source_variables),
        setter_place: setter_place.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn analyse(source: &str, profile: &str) -> SourceCommandBindings {
        let environment = tcl_registry::model::ingress::static_context_for(profile);
        let registry = environment.commands();
        SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::for_profile(registry.profile()),
            registry,
            super::super::SourceAnalysisOptions {
                invocation_dialect: registry
                    .profile()
                    .map(tcl_registry::InvocationDialect::of_profile),
                native_compilation: crate::environment_ingress::authoring_native_compilation(),
                ..super::super::SourceAnalysisOptions::default()
            },
        )
    }

    #[test]
    fn original_rhs_read_and_setter_retain_their_separate_contexts() {
        for profile in ["tcl8.6", "tcl9.1"] {
            let source = "set x 0; set x [expr {$x+1}]";
            let bindings = analyse(source, profile);
            let setter = bindings
                .invocation_at_source("set", u32::try_from(source.rfind("set").unwrap()).unwrap());
            let observations = setter.sole_rhs_read_store_observations().expect(profile);
            assert_eq!(observations.len(), 1, "{profile}");
            let observation = &observations[0];
            assert_eq!(
                observation.read_place(),
                observation.setter_place(),
                "{profile}"
            );
            let environment = tcl_registry::model::ingress::static_context_for(profile);
            assert_ne!(
                observation
                    .read_context()
                    .contents_native_numeric_category_at(
                        observation.read_place(),
                        environment.commands()
                    ),
                Some(tcl_registry::TclType::Int),
                "{profile}"
            );
            assert_eq!(
                observation.operand().span.start(),
                u32::try_from(source.find("$x").unwrap()).unwrap()
            );
        }
    }

    #[test]
    fn unknown_physical_generation_cannot_construct_a_read_store_receipt() {
        let source = "proc f {flag} {set x 0; if {$flag} {unset x; set x 0}; set x [expr {$x+1}]}";
        let bindings = analyse(source, "tcl8.6");
        let setter = bindings
            .invocation_at_source("set", u32::try_from(source.rfind("set").unwrap()).unwrap());
        assert!(setter.sole_rhs_read_store_observations().is_none());
        let environment = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let start = u32::try_from(source.rfind("$x").unwrap()).unwrap();
        let reads = bindings.variable_accesses_in_span(tcl_lexer::Span::new(start, start + 2));
        assert_eq!(reads.len(), 1);
        assert!(reads[0].context_alternatives().iter().all(|context| {
            reads[0]
                .place_in_context(context, environment.commands())
                .cell
                .as_ref()
                .is_none_or(|cell| cell.generation == crate::place::CellGeneration::Unknown)
        }));
    }

    #[test]
    fn callbacks_templates_and_different_destinations_withdraw_pairing() {
        for source in [
            "set x 0; set y [expr {$x+1}]",
            "set x 0; set x \"prefix[expr {$x+1}]\"",
            "set x 0; proc observe args {}; trace add variable x read observe; set x [expr {$x+1}]",
            "set x 0; proc observe args {}; trace add variable x write observe; set x [expr {$x+1}]",
        ] {
            let bindings = analyse(source, "tcl8.6");
            let setter = bindings
                .invocation_at_source("set", u32::try_from(source.rfind("set").unwrap()).unwrap());
            assert!(
                setter.sole_rhs_read_store_observations().is_none(),
                "{source}"
            );
        }
    }
}
