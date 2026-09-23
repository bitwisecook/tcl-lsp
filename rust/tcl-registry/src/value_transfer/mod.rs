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

//! Value transfers — what the registry tells the analyser about one command
//! invocation on the value axis
//! (`docs/design/compiler/value-transfers.md`).
//!
//! The registry owns the command-specific meaning: what an invocation
//! computes, which storage it writes, in what order, and which bindings and
//! target semantics the answer depends on. It declares that meaning as a
//! [`CommandSemantics`] specialisation on a [`crate::CommandSpec`], a
//! [`crate::SubCommand`], or a [`crate::forms::CommandForm`] — or derives
//! one from a descriptor that states the same operation
//! ([`declaration::resolve_semantics`]). The analyser owns the generic
//! operations — proving a word's value, resolving a place, reading the
//! prior fact, running the solver, validating and joining answers — and
//! hands the specialisation a read-only [`inputs::AnalysisInputs`] view.
//! Nothing here reaches into SSA, and nothing here is selected by a
//! command name outside this crate.
//!
//! The evaluation route a specialisation names ([`route::EvalRoute`]) is a
//! declared capability; purity never selects one
//! (`docs/design/compiler/value-evaluation.md`).

pub mod answers;
pub mod builtins;
pub mod cell_update;
pub mod cell_write;
pub mod const_ops;
pub mod context;
pub mod declaration;
pub mod decline;
pub mod inputs;
pub mod iteration;
pub mod keyed_update;
pub mod lift;
pub mod literal;
pub mod route;
pub mod unbind;

pub use answers::{
    Binder, BinderName, BindingKind, BodyPlan, CompletionOutcome, CompletionPath,
    CompletionProtocol, DependencyEvidence, EvalAnswer, ExactValue, ExactValueOrUnavailable,
    Existence, ExistenceOutcome, ExistenceTransfer, ExitRule, FactBounds, HandlerMatch,
    HandlerPlan, InvocationOutcome, IterableKind, IterationPlan, NumericValue, PlanAnswer,
    RangeModel, Reconcile, RepresentationEvidence, RouteIdentity, ScriptRegion, SegmentFacts,
    SelectionContract, SelectionFact, StoreOutcome, TaintTransfer, TemplateWordPlan,
    TransferAnswer, TypeFacts, ValueShape, VariableRead,
};
pub use const_ops::{ConstOps, ConstValue, Needs, Representation, TargetSemantics, WorkUnits};
pub use context::{AnalysisContext, BindingEvidence, BindingIdentity, Budget};
pub use declaration::{
    DeclarationScope, DerivedSemantics, ResolvedSemantics, SemanticsDeclaration, SemanticsOrigin,
    resolve_semantics,
};
pub use decline::{AnalysisTier, Axis, BudgetLimit, DeclineReason, NoRouteReason};
pub use inputs::{
    AnalysisInputs, BodyRegion, DomainFact, EvaluationState, FactDomain, FactView,
    InvocationLayout, NestedPolicy, OperandId, OperandView, PlaceKind, PlaceRef,
    ResolvedInvocationView, TargetId, ValueIdentity, WordPart, WordStructure,
};
pub use lift::{LiftedAnswer, PinnedInputs, evaluate_lifted, finite_inputs};
pub use literal::{LiteralInputs, evaluate_literal};
pub use route::{EvalRoute, EvaluatorCapability, EvaluatorOwner, LanguageProfileId, NativeEvalId};

/// What a registry-owned specialisation supplies for one invocation.
///
/// Returned plans and facts are preferred to callbacks because a plan can
/// be validated, replayed, cached, and consumed by more than one frontend.
/// The specialisation composes the analyser's generic operations through
/// [`AnalysisInputs`]; it never receives the analyser itself.
pub trait CommandSemantics: Sync + Send {
    /// A stable identity for the specialisation: the memo key's
    /// `SpecialisationId` and the inventory's spelling.
    fn identity(&self) -> &'static str;

    /// The declared evaluator route.
    fn route(&self) -> EvalRoute;

    /// Bodies, scopes, binders, control and completion protocol, declared
    /// structural effects — consumed at construction time.
    fn structure(&self, input: &dyn AnalysisInputs) -> PlanAnswer {
        let _ = input;
        PlanAnswer::NoStructure
    }

    /// The targets whose incoming value and existence this invocation's
    /// evaluation reads. The derived cell updates need no override.
    fn incoming_targets(&self, input: &dyn AnalysisInputs) -> Vec<TargetId> {
        match self.structure(input) {
            PlanAnswer::CellReadModifyWrite { target, .. } => vec![target],
            _ => Vec::new(),
        }
    }

    /// The variables this invocation's evaluation reads by name, through
    /// [`AnalysisInputs::variable`] rather than through an operand: the
    /// `$name` operands an expression reads itself. The lift counts their
    /// finite views among the evaluation's distinct inputs
    /// ([`lift::finite_inputs`]). The default reads none.
    fn variable_reads(&self, input: &dyn AnalysisInputs) -> Vec<String> {
        let _ = input;
        Vec::new()
    }

    /// The abstract transfer for one fact domain: a delta the owning solver
    /// validates and applies. A transfer that evaluates charges the same
    /// budget as [`Self::evaluate`].
    fn transfer(
        &self,
        domain: FactDomain,
        input: &dyn AnalysisInputs,
        budget: &mut Budget,
    ) -> TransferAnswer {
        let _ = (domain, input, budget);
        TransferAnswer::Generic
    }

    /// The exact evaluation over the declared route, under a budget. The
    /// default declines with the route's own reason: a specialisation with
    /// a registry-owned direct route overrides this; an expression route is
    /// run by the driver's engine adapter; a transitional direct route is
    /// run by the driver's handler until the slice that retires it.
    fn evaluate(&self, input: &dyn AnalysisInputs, budget: &mut Budget) -> EvalAnswer {
        let _ = (input, budget);
        match self.route() {
            EvalRoute::None { reason } => EvalAnswer::Declined(DeclineReason::NoRoute(reason)),
            EvalRoute::Direct { .. }
            | EvalRoute::Expression { .. }
            | EvalRoute::Implementation(_) => EvalAnswer::Declined(DeclineReason::Unsupported),
        }
    }
}

impl std::fmt::Debug for dyn CommandSemantics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommandSemantics")
            .field("identity", &self.identity())
            .field("route", &self.route())
            .finish()
    }
}
