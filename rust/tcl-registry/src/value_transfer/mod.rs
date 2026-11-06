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
pub mod body;
pub mod builtins;
pub mod cell_update;
pub mod cell_write;
pub mod completion;
pub mod conditional_list_append;
pub mod conditional_list_index;
pub mod const_ops;
pub mod context;
pub mod declaration;
pub mod declared;
pub mod decline;
pub mod destructure;
pub mod inputs;
pub mod irules;
pub mod iteration;
pub mod keyed_update;
pub mod lift;
pub mod list_update;
pub mod literal;
pub mod path;
mod publication;
pub mod reference_body;
pub mod regex;
pub mod route;
pub mod scope_alias;
pub mod selection;
pub mod tcllib;
pub mod template;
pub mod unbind;

pub use answers::{
    AliasFrame, Binder, BinderName, BindingKind, BodyPlan, CaseArms, CompletionOutcome,
    CompletionPath, CompletionProtocol, DependencyEvidence, EvalAnswer, ExactValue,
    ExactValueOrUnavailable, Existence, ExistenceOutcome, ExistenceTransfer, ExitRule, FactBounds,
    HandlerMatch, HandlerPlan, InvocationOutcome, IterableKind, IterationPlan, ListGroup,
    NumericValue, PlanAnswer, RangeModel, Reconcile, RepresentationEvidence, RouteIdentity,
    ScopeAliasPlan, ScriptRegion, SegmentFacts, SelectionContract, SelectionFact, StoreOutcome,
    TaintTransfer, TemplateWordPlan, TransferAnswer, TypeFacts, ValueShape, VariableRead,
    validate_outcome,
};
pub use conditional_list_append::{
    ConditionalSourceListAppend, ConditionalSourceListAppendDirectPhase,
};
pub use conditional_list_index::ConditionalSourceLiteralListIndex;
pub use const_ops::{ConstOps, ConstValue, Needs, Representation, TargetSemantics, WorkUnits};
pub use context::{AnalysisContext, BindingEvidence, BindingIdentity, Budget, EvaluatorGeneration};
pub use declaration::{
    DeclarationScope, DerivedSemantics, ResolvedSemantics, SemanticsDeclaration, SemanticsOrigin,
    resolve_semantics,
};
pub use declared::{
    DeclaredEffect, DeclaredEvaluation, DeclaredImplementation, DeclaredIteration,
    DeclaredSemantics, DeclaredStores, DeclaredStructure, ExtensionArtefact, IterableWord,
    OutcomeKind, PACK_DIRECT_EVALUATORS, SemanticType, pack_direct_evaluator,
};
pub use decline::{AnalysisTier, Axis, BudgetLimit, DeclineReason, NoRouteReason};
pub use inputs::{
    AnalysisInputs, BodyRegion, DomainFact, EvaluationState, FactDomain, FactView,
    InvocationLayout, NestedPolicy, OperandId, OperandView, ParameterDefault, PlaceKind, PlaceRef,
    ResolvedInvocationView, TargetId, ValueIdentity, WordPart, WordStructure, WrittenPlace,
    written_in,
};
pub use iteration::{LOOP_ABSORBED, LoopResult, LoopStep};
pub use lift::{LiftedAnswer, PinnedInputs, evaluate_lifted, finite_inputs};
pub use literal::{LiteralInputs, evaluate_literal};
pub use route::{
    CompletionSupport, ContextDependency, DeclaredInput, EvalRoute, EvaluatorCapability,
    EvaluatorOwner, Exactness, HostKind, ImplementationBudget, ImplementationIdentity,
    LanguageProfileId, NativeEvalId, OptionEvaluation,
};

/// Representation relations under a successful original source handler.
/// Cell relations additionally assume ordinary cells without write observers;
/// a trace or alias may replace the value. These descriptions grant no
/// dispatch, completion, observer absence or physical-object proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionalSourceRepresentation {
    /// A successful handler produces an integer representation for its result; when present, the
    /// same result is stored in the named read-modify-write target.
    IntegerResult {
        /// Conditional store target, if this is a cell update.
        target: Option<TargetId>,
    },
    /// A successful write forwards the captured value object to its target
    /// and result. This does not infer a representation from its bytes.
    ForwardValue {
        /// Source operand, captured before the handler runs.
        value: OperandId,
        /// Target of the conditional store.
        target: TargetId,
    },
}

/// A conditional prior-cell dependency and numeric store from an audited
/// original increment handler. This assumes successful integer parsing and an
/// ordinary cell without observers; it grants no dispatch, completion, absence
/// of traces, object sharing or physical mutation proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConditionalSourceTargetUpdate {
    target: TargetId,
    prior: crate::TclType,
    stored: crate::TclType,
    numbers: tcl_dialect::NumberSyntax,
}

impl ConditionalSourceTargetUpdate {
    /// Whether an exact ASCII source value cannot be parsed by the selected
    /// C Tcl integer grammar. Unknown encoding and large valid integers stay
    /// conditional; this does not prove completion or representation.
    #[must_use]
    pub fn definitely_rejects_literal(self, bytes: &[u8]) -> bool {
        if !bytes.is_ascii() {
            return false;
        }
        let Ok(text) = std::str::from_utf8(bytes) else {
            return false;
        };
        !matches!(
            tcl_syntax::number::parse_whole_with(
                text,
                tcl_syntax::number::ParseFlags {
                    integer_only: true,
                    ..tcl_syntax::number::ParseFlags::for_syntax(self.numbers)
                },
            ),
            Some(tcl_syntax::number::Number::Int(_) | tcl_syntax::number::Number::Big { .. })
        )
    }

    /// The source-owned cell whose previous value is read and updated.
    #[must_use]
    pub const fn target(self) -> TargetId {
        self.target
    }

    /// The prior value is consumed as an integer under these premises.
    #[must_use]
    pub const fn prior_representation_requirement(self) -> crate::TclType {
        self.prior
    }

    /// The conditional store contains an integer representation.
    #[must_use]
    pub const fn stored_representation(self) -> crate::TclType {
        self.stored
    }
}

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

    /// A narrowly audited source-only representation relation. The returned
    /// shape is conditional on the original handler completing successfully;
    /// it does not evaluate operands or establish any native permission.
    fn conditional_source_representation(
        &self,
        input: &dyn AnalysisInputs,
    ) -> Option<ConditionalSourceRepresentation> {
        let _ = input;
        None
    }

    /// Conditional source phase of an audited ordinary-object paired loop.
    /// This does not provide a reached handler or body entry.
    fn conditional_source_paired_loop_handler(
        &self,
    ) -> Option<crate::iteration_entry::ConditionalSourcePairedLoopHandler> {
        None
    }

    /// A separately audited source-only prior-target update. The default
    /// declines; this never supplies a reached store or execution authority.
    fn conditional_source_target_update(
        &self,
        input: &dyn AnalysisInputs,
    ) -> Option<ConditionalSourceTargetUpdate> {
        let _ = input;
        None
    }

    /// Bodies, scopes, binders, control and completion protocol, declared
    /// structural effects — consumed at construction time.
    fn structure(&self, input: &dyn AnalysisInputs) -> PlanAnswer {
        let _ = input;
        PlanAnswer::NoStructure
    }

    /// The frame a scope-alias declaration links the locals its `VarWrite`
    /// operands name into, whatever the invocation: the frame its
    /// [`PlanAnswer::ScopeAlias`] plan states. `None` for every other
    /// specialisation.
    fn alias_frame(&self) -> Option<AliasFrame> {
        None
    }

    /// The targets whose incoming value and existence this invocation's
    /// evaluation reads. The derived cell updates need no override.
    fn incoming_targets(&self, input: &dyn AnalysisInputs) -> Vec<TargetId> {
        match self.structure(input) {
            PlanAnswer::CellReadModifyWrite { target, .. } => vec![target],
            _ => Vec::new(),
        }
    }

    /// The targets this invocation's ordered stores may name, in operand
    /// order: every operand the resolver gives the `VarWrite` role, and the
    /// target a cell-update plan names. The driver validates each outcome
    /// against them ([`answers::validate_outcome`]) before anything is
    /// published.
    fn store_targets(&self, input: &dyn AnalysisInputs) -> Vec<TargetId> {
        let mut targets: Vec<TargetId> = input
            .invocation()
            .operands_with_role(crate::arg_role::ArgRole::VarWrite)
            .map(TargetId)
            .collect();
        if let PlanAnswer::CellReadModifyWrite { target, .. } = self.structure(input)
            && !targets.contains(&target)
        {
            targets.push(target);
            targets.sort_unstable();
        }
        targets
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
    /// a direct route overrides this; an expression route is run by the
    /// driver's engine adapter.
    fn evaluate(&self, input: &dyn AnalysisInputs, budget: &mut Budget) -> EvalAnswer {
        let _ = (input, budget);
        match self.route() {
            EvalRoute::None { reason } => EvalAnswer::Declined(DeclineReason::NoRoute(reason)),
            EvalRoute::Direct { .. }
            | EvalRoute::Expression { .. }
            | EvalRoute::Implementation(_) => EvalAnswer::Declined(DeclineReason::Unsupported),
        }
    }

    /// The pack declaration this specialisation is, when it is one: what
    /// the host plan binds a declared implementation's body into.
    fn as_declared(&self) -> Option<&declared::DeclaredSemantics> {
        None
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

#[cfg(test)]
mod conditional_source_representation_tests {
    use super::*;

    #[test]
    fn conditional_increment_target_update_requires_audited_c_profile_and_shape() {
        let update = cell_update::CellUpdateSemantics {
            update: crate::native_lowering::CellUpdate::Increment,
            creates_absent: None,
        };
        let append = cell_update::CellUpdateSemantics {
            update: crate::native_lowering::CellUpdate::ListAppend,
            creates_absent: None,
        };
        for version in tcl_dialect::TclVersion::ALL {
            let profile =
                crate::model::ingress::resolve_environment(version.dialect_profile_name())
                    .unit_profile();
            let one = LiteralInputs::new("incr", None, &["x"], Some(profile));
            let relation = update
                .conditional_source_target_update(&one)
                .expect("audited source relation");
            assert_eq!(relation.target(), TargetId(OperandId(0)));
            assert_eq!(
                relation.prior_representation_requirement(),
                crate::TclType::Int
            );
            assert_eq!(relation.stored_representation(), crate::TclType::Int);
            assert!(relation.definitely_rejects_literal(b"abc"));
            assert!(relation.definitely_rejects_literal(b"1 2"));
            assert!(relation.definitely_rejects_literal(b"Inf"));
            assert!(relation.definitely_rejects_literal(b"NaN"));
            assert!(!relation.definitely_rejects_literal(b"0"));
            assert!(!relation.definitely_rejects_literal(b"9223372036854775807"));
            assert!(!relation.definitely_rejects_literal(b"922337203685477581000000000000000"));
            assert!(!relation.definitely_rejects_literal(&[0xff]));
            let explicit = LiteralInputs::new("incr", None, &["x", "1"], Some(profile));
            assert!(update.conditional_source_target_update(&explicit).is_none());
            assert!(append.conditional_source_target_update(&one).is_none());
        }
        let f5 = crate::model::ingress::resolve_environment("f5-irules").unit_profile();
        assert!(
            update
                .conditional_source_target_update(&LiteralInputs::new(
                    "incr",
                    None,
                    &["x"],
                    Some(f5)
                ))
                .is_none()
        );
        let jim = crate::model::ingress::resolve_environment("jim").unit_profile();
        assert!(
            update
                .conditional_source_target_update(&LiteralInputs::new(
                    "incr",
                    None,
                    &["x"],
                    Some(jim)
                ))
                .is_none()
        );
        assert!(
            update
                .conditional_source_target_update(&LiteralInputs::new("incr", None, &["x"], None))
                .is_none()
        );
    }

    #[test]
    fn audited_result_relations_keep_source_value_and_arity_distinct() {
        let unknown_length =
            LiteralInputs::new("string", Some("length"), &["$s"], None).with_dynamic(OperandId(1));
        assert_eq!(
            builtins::STRING_LENGTH.conditional_source_representation(&unknown_length),
            Some(ConditionalSourceRepresentation::IntegerResult { target: None })
        );
        let missing = LiteralInputs::new("string", Some("length"), &[], None);
        assert_eq!(
            builtins::STRING_LENGTH.conditional_source_representation(&missing),
            None
        );
        let literal = LiteralInputs::new("string", Some("length"), &["abc"], None);
        assert_eq!(
            builtins::STRING_LENGTH.conditional_source_representation(&literal),
            None,
            "the native compiler can emit a pooled literal result"
        );
        let unproven_literal = LiteralInputs::new("string", Some("length"), &["abc"], None)
            .with_unproven(OperandId(1));
        assert_eq!(
            builtins::STRING_LENGTH.conditional_source_representation(&unproven_literal),
            None,
            "unavailable value facts cannot change original word kind"
        );
        assert_eq!(
            builtins::STRING_RANGE.conditional_source_representation(&unknown_length),
            None
        );

        let write = LiteralInputs::new("set", None, &["x", "$value"], None)
            .with_role(OperandId(0), crate::ArgRole::VarWrite)
            .with_unproven(OperandId(1));
        assert_eq!(
            cell_write::CELL_WRITE.conditional_source_representation(&write),
            Some(ConditionalSourceRepresentation::ForwardValue {
                target: TargetId(OperandId(0)),
                value: OperandId(1),
            })
        );
        let read = LiteralInputs::new("set", None, &["x"], None)
            .with_role(OperandId(0), crate::ArgRole::VarRead);
        assert_eq!(
            cell_write::CELL_WRITE.conditional_source_representation(&read),
            None
        );

        let update = cell_update::CellUpdateSemantics {
            update: crate::native_lowering::CellUpdate::Increment,
            creates_absent: None,
        };
        let args = LiteralInputs::new("incr", None, &["x"], None);
        assert_eq!(
            update.conditional_source_representation(&args),
            Some(ConditionalSourceRepresentation::IntegerResult {
                target: Some(TargetId(OperandId(0))),
            })
        );
        let append = cell_update::CellUpdateSemantics {
            update: crate::native_lowering::CellUpdate::ListAppend,
            creates_absent: None,
        };
        assert_eq!(append.conditional_source_representation(&args), None);
        let extra = LiteralInputs::new("incr", None, &["x", "1", "2"], None);
        assert_eq!(update.conditional_source_representation(&extra), None);
    }
}
