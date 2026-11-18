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

//! The value-transfer driver: the analyser's half of
//! `docs/design/compiler/value-transfers.md`.
//!
//! The registry owns what an invocation computes
//! ([`tcl_registry::value_transfer::CommandSemantics`]); this module owns the
//! generic operations the specialisation composes — proving a word's value
//! from the SCCP lattice, resolving a storage place, reading the fact that
//! holds there, and applying a validated answer to a definition. It matches
//! no command name: every statement is resolved through the invocation
//! resolver, and the registry's declaration for the resolved command,
//! subcommand, and form decides what runs.
//!
//! Every declared direct route is implemented in the registry
//! ([`NativeEvalId::owner`](tcl_registry::value_transfer::NativeEvalId::owner)).
//! The expression route is run here by
//! construction: the registry assembles the argument words
//! ([`ExpressionRoute::assemble`]) and the shared engine evaluates the
//! expression over this module's lattice services
//! ([`crate::tcl_expr_eval::ExprServices`]).

use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use tcl_lexer::{LexerConfig, Span, TokenType};
use tcl_registry::hooks::LoweringHookId;
use tcl_registry::value_transfer::{
    AnalysisContext, AnalysisTier, BindingIdentity, BindingKind, Budget, CompletionOutcome,
    DeclineReason, DependencyEvidence, EvalRoute, ExactValueOrUnavailable, Existence,
    ExistenceOutcome, FactBounds, FactView, InvocationOutcome, LiftedAnswer, NestedPolicy,
    ParameterDefault, PlaceKind, PlaceRef, ResolvedInvocationView, StoreOutcome,
};
#[cfg(test)]
use tcl_registry::value_transfer::{
    AnalysisInputs, BodyRegion, CommandSemantics, EvalAnswer, EvaluationState, ExactValue,
    FactDomain, InvocationLayout, OperandId, TargetId, TypeFacts, ValueIdentity, WordStructure,
};
use tcl_registry::{
    CommandRegistry, InvocationWord, InvocationWordKind, InvocationWords, ResolvedInvocation,
};

use crate::analyses::{ConstValue, LatticeValue};
#[cfg(test)]
use crate::expr_ast::ExprNode;
use crate::ir::{CommandTokens, Statement};
use crate::sccp::{BuiltinFoldInputs, FoldTrust, TraceInputs};
use crate::ssa::{SsaFunction, SsaStatement, Symbol, ValueKey, Version};
use crate::tcl_expr_eval::FoldPolicy;

mod original_diagnostic_values;

pub use original_diagnostic_values::OriginalDiagnosticValues;

mod argument_words;
mod compiled_constants;
mod context_key;
mod enumeration_state;
mod existence;
mod expressions;
mod folded_type;
mod lattice_inputs;
mod procedure_calls;
mod protected_scripts;
mod queries;
mod run_records;
mod scripts;
mod statements;

#[cfg(test)]
use argument_words::word_of;
use argument_words::{OperandSource, typed_node_commands};
pub(crate) use argument_words::{literal_token_value, recorded_word_value, view_of};
pub use context_key::{AnalysisContextKey, ConnectionScoped};
pub(crate) use enumeration_state::StateStep;
use expressions::Expression;
#[cfg(test)]
use expressions::expression_parse_context;
#[cfg(test)]
use expressions::truth_of;
pub(crate) use expressions::{
    decide_condition_detached, evaluate_expression_detached, evaluate_expression_in_module,
};
pub use folded_type::FoldedType;
use lattice_inputs::Words;
pub(crate) use lattice_inputs::{
    const_text, const_to_exact, exact_to_const, exact_to_lattice, variable_name,
};
pub(crate) use procedure_calls::summary_steps;
pub(crate) use queries::{
    DictBinder, StructureInputs, WordForm, cell_update_range_model,
    dict_body_operand_with_metadata_context, dict_body_with_metadata_context, literal_keyed_update,
    literal_selection, proven_return_word_value_with_facts,
    proven_substituted_word_value_with_facts, proven_word_value_with_facts,
    resolved_body_absorbs_completion, resolved_cell_update, resolved_iteration_plan,
    resolved_literal_semantics, selected_iteration_plan, source_template_plan,
    typed_incr_creates_absent, whole_variable_operand,
};
#[cfg(test)]
pub(crate) use queries::{LiteralSelection, statement_selection};
pub use queries::{
    SourceWord, StatementId, SubstitutionHost, literal_template_plan, proven_return_word_value,
    proven_substituted_word_value, proven_word_value, return_command_tokens, substitution_calls,
};
#[cfg(test)]
pub(crate) use queries::{dict_body, dict_body_operand};
use run_records::request_budget;

/// One statement's route and answer, as the Explorer's `sccp` view shows
/// it: which route the resolved invocation declared, and whether it
/// evaluated, declined with which reason, or is still pending.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteExplanation {
    /// The statement's span.
    pub span: Span,
    /// The resolved command.
    pub command: String,
    /// The declared route, or why there is none.
    pub route: String,
    /// The answer on the last solver pass over the statement.
    pub answer: String,
    /// What the statement stores on each completion path its evaluation
    /// found, or — for a command whose transfer lists them and whose
    /// evaluation declined — the existence outcomes each path states.
    pub paths: Vec<PathExplanation>,
}

/// How an invocation completes on one path of an explanation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathCompletion {
    /// `TCL_OK`.
    Normal,
    /// `TCL_ERROR`, after this many stores where the evaluation counted them
    /// and with none given where a transfer lists the path without placing
    /// the failing step.
    Error(Option<usize>),
    /// Another completion code.
    Code(i64),
    /// One of several codes a transfer lists.
    Several(usize),
    /// Any completion.
    Any,
}

impl PathCompletion {
    /// The completion an evaluated outcome has.
    fn of(completion: &CompletionOutcome) -> Self {
        match completion {
            CompletionOutcome::Normal => Self::Normal,
            CompletionOutcome::Code { code, .. } => Self::Code(code.as_int()),
            CompletionOutcome::Error { written, .. } => Self::Error(Some(*written)),
        }
    }

    /// The completion a transfer's path lists.
    fn listed(domain: tcl_registry::completion::CompletionCodeDomain) -> Self {
        use tcl_registry::completion::{CompletionCode, CompletionCodeDomain};
        match domain {
            CompletionCodeDomain::Any => Self::Any,
            CompletionCodeDomain::Exact([CompletionCode::Ok]) => Self::Normal,
            CompletionCodeDomain::Exact([CompletionCode::Error]) => Self::Error(None),
            CompletionCodeDomain::Exact(codes) => Self::Several(codes.len()),
        }
    }

    /// Whether this is an error, counted or not.
    fn is_error(self) -> bool {
        matches!(self, Self::Error(_))
    }

    /// The completion as an explanation spells it.
    #[must_use]
    pub fn label(self) -> String {
        match self {
            Self::Normal => "normal".to_owned(),
            Self::Error(None) => "error".to_owned(),
            Self::Error(Some(written)) => format!(
                "error after {written} store{}",
                if written == 1 { "" } else { "s" }
            ),
            Self::Code(code) => format!("code {code}"),
            Self::Several(count) => format!("{count} codes"),
            Self::Any => "any completion".to_owned(),
        }
    }
}

/// One completion path of a statement, for an explanation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathExplanation {
    /// How the invocation completes on the path.
    pub completion: PathCompletion,
    /// What it stores on it, in execution order.
    pub stores: Vec<String>,
}

/// How many times the run dispatched to each route family, nested entries
/// included: a direct-routed `call_def` (the typed `incr` and every call)
/// or `run_script` counts in [`Self::direct`]; a `run_script` that resolves
/// to `expr`, `evaluate_assign_expr` and `evaluate_condition` count in
/// [`Self::expression`]; an implementation-routed `call_def` or
/// `run_script` counts in [`Self::implementation`]. Carries no span — a
/// route entry has no one statement of its own once nesting is counted —
/// so `lattice_rebase.rs` does not touch it. The Explorer's `sccp` view
/// renders it as `routes entered: direct N · expression M ·
/// implementation K`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RouteTally {
    /// Entries into a registry-owned direct evaluator.
    pub direct: u32,
    /// Entries into the shared expression engine.
    pub expression: u32,
    /// Entries into a declared `EvaluatorCapability` implementation.
    pub implementation: u32,
}

/// The lattice at one program point: the versions a statement reads, the
/// value each version holds, and the function they belong to.
type Lattice<'a, S1, S2> = (
    &'a HashMap<Symbol, Version, S1>,
    &'a HashMap<ValueKey, LatticeValue, S2>,
    &'a SsaFunction,
);

/// The `catch` of a flattened region, and where the state its script runs over
/// is: the words the analysis build keeps beside the statement that ends the
/// region ([`crate::cfg::CatchEnd`]).
pub(crate) struct CatchEndRecord {
    /// The block before the body.
    pub(crate) entry: crate::cfg::BlockId,
    /// The `catch` as a call, in the form it has when it is not flattened.
    pub(crate) call: Statement,
}

/// The driver's per-run state: the registry the unit resolves against, the
/// whole-module trust fact when the caller holds one, the fold policy, the
/// analysis context every answer is memoised under, and the route
/// explanations the run records for the statement it is evaluating.
pub(crate) struct LatticeDriver<'a> {
    registry: &'a CommandRegistry,
    /// Terminal unavailable, explicit standalone, or the actual retained generation.
    metadata_context: Option<crate::registry_invocation::InvocationMetadataSelection<'a>>,
    source_metadata_input: Option<&'a crate::analyser::ResolvedAnalysisInput>,
    folds: Option<BuiltinFoldInputs<'a>>,
    policy: FoldPolicy,
    expression_preparations:
        HashMap<Span, Option<&'a crate::command_binding::SourceExpressionPreparation>>,
    context: AnalysisContext,
    lexer_config: LexerConfig,
    /// Whether every command whose lowering yields a typed assignment
    /// (`set`) still denotes its builtin — the named half of the trust
    /// fact, as the chain fold asks it.
    typed_assignment: bool,
    /// How deep the nested-substitution service is: each `[…]` a route or
    /// an expression evaluates enters one level, bounded by the evaluation
    /// depth.
    nesting: Cell<u32>,
    /// The statement being evaluated, when the solver said which.
    explaining: Cell<Option<Span>>,
    /// Whether the statement being evaluated sits where a throw leaves
    /// from — a block of a `catch` or `try` body — so that a statement that
    /// certainly raises answers the state its stores left rather than
    /// widening.
    throwing: Cell<bool>,
    /// Whether a command a word substitutes raised an error since the last
    /// invocation was evaluated: Tcl substitutes every word before it runs
    /// the command, so the command never ran.
    word_error: Cell<bool>,
    /// The last explanation recorded per statement.
    explanations: RefCell<BTreeMap<(u32, u32), RouteExplanation>>,
    /// The folded type of each definition the last evaluation of its
    /// statement stated one for ([`crate::sccp::SccpResult::folded_types`]).
    folded: RefCell<HashMap<ValueKey, FoldedType>>,
    /// The definitions the last evaluation of their statement preserved,
    /// with the version each preserved ([`crate::sccp::SccpResult::preserved`]).
    preserved: RefCell<HashMap<ValueKey, Version>>,
    /// The statements whose last evaluation certainly raised where a handler
    /// is thrown to ([`crate::sccp::SccpResult::raised`]).
    raised: RefCell<HashSet<(crate::cfg::BlockId, usize)>>,
    /// Whether the run reads how the procedure completes
    /// ([`crate::sccp::ModuleRun::reads_exits`]): a statement that certainly
    /// raises ends its path in every block.
    reads_exits: bool,
    /// The statements whose last evaluation certainly raised where no
    /// handler takes the throw ([`crate::sccp::RunCompletion::raises`]).
    uncaught: RefCell<HashSet<(crate::cfg::BlockId, usize)>>,
    /// Whether the current sweep applied a call whose completion no re-run
    /// decided ([`crate::sccp::RunCompletion::undecided`]).
    undecided: Cell<bool>,
    /// The run's route-entry counts so far.
    tally: Cell<RouteTally>,
    /// The run's request budget: every evaluation of this run charges
    /// through it, so a run whose evaluations spend it declines the rest.
    request: RefCell<Budget>,
    /// The current solver pass's share of the request.
    iteration: RefCell<Budget>,
    /// The existence rung at the point being evaluated, one fact per
    /// symbol: the state before the statement, or at its block's exit for
    /// the terminator. `None` for a run that computes no existence (a
    /// re-run, a detached evaluation), whose reads are `Unavailable`.
    existence: RefCell<Option<Vec<Existence>>>,
    /// The rung's slots past the SSA's symbols: a place only an existence
    /// query names (`if {[info exists x]}` with no other `x`).
    existence_places: RefCell<HashMap<String, Symbol>>,
    /// Per slot, whether the place is externally mutable — qualified,
    /// aliased, traced, an instance variable, a connection's name or a
    /// special variable of the initial global frame: an existence query
    /// about one decides nothing.
    existence_external: RefCell<Vec<bool>>,
    /// Whether the run is the document's initial global frame, where an
    /// existence query about a registry special variable decides nothing:
    /// the host, not the script, binds it.
    existence_initial_global: bool,
    /// The function's flattened `catch` regions, by the name of the block that
    /// ends each ([`crate::cfg::Function::catch_ends`]).
    catch_ends: RefCell<HashMap<String, CatchEndRecord>>,
    /// The existence rung where the scripts of the statement being evaluated
    /// start, when it is not the point the statement stands at: before the
    /// marker that states what a statement's scripts may write, and at the
    /// exit of the block before a flattened `catch` body. An existence read
    /// while it is set answers from it ([`Self::existence_fact`]).
    body_entry: RefCell<Option<Vec<Existence>>>,
    /// The module's procedures, when the run's caller holds them.
    module: Option<&'a crate::interprocedural::ModuleProcedures<'a>>,
    /// What a call to a procedure of the module gives the run
    /// ([`crate::sccp::ModuleLevel`]).
    level: crate::sccp::ModuleLevel,
    /// The function the run analyses, by qualified name: where a procedure
    /// name it spells is looked up from.
    function: &'a str,
    body_id: Option<crate::sccp::AnalysisBody>,
    /// Whether the run read the module's procedures, or would have read them
    /// had its caller held them ([`crate::sccp::SccpResult::reads_module`]).
    reads_module: Cell<bool>,
}

/// The lattice value of one member-wise evaluation: the constant `pick`
/// answers for each outcome, one constant or a finite set of them, and
/// `Overdefined` when any outcome has none.
fn lattice_of_outcomes(
    outcomes: &[Box<InvocationOutcome>],
    pick: impl Fn(&InvocationOutcome) -> Option<LatticeValue>,
) -> LatticeValue {
    let mut consts: Vec<ConstValue> = Vec::with_capacity(outcomes.len());
    for outcome in outcomes {
        let Some(LatticeValue::Const(c)) = pick(outcome) else {
            return LatticeValue::Overdefined;
        };
        if !consts.contains(&c) {
            consts.push(c);
        }
    }
    match consts.len() {
        0 => LatticeValue::Overdefined,
        1 => LatticeValue::Const(consts.pop().unwrap()),
        _ => LatticeValue::constset(consts),
    }
}

/// What one statement does to one place's existence
/// (`docs/design/compiler/value-transfers.md` § *Existence*): afterwards the
/// place holds `Set`'s fact whatever it held before, or the join of what it
/// held with `Join`'s. A write is `Set(Bound(_))`, an unbind
/// `Set(Unbound)`, a preserve `Join(Pending)` and a may-write
/// `Join(Bound(_))`; a statement whose outcome waits on an input the solver
/// has not reached is `Set(Pending)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExistenceStep {
    /// The fact afterwards, whatever the place held.
    Set(Existence),
    /// Joined with what the place held.
    Join(Existence),
}

impl ExistenceStep {
    /// A statement whose effect on the place the analysis cannot state — a
    /// command with no existence transfer, a rebound head, a declined
    /// answer: the place may have been written or destroyed, so its fact
    /// widens, as its value does.
    pub(crate) const UNKNOWN: Self = Self::Set(Existence::MayBound);
    /// The place is left as it was.
    pub(crate) const PRESERVE: Self = Self::Join(Existence::Pending);
    /// The outcome is not known yet.
    pub(crate) const PENDING: Self = Self::Set(Existence::Pending);

    /// The step a storage outcome's existence delta takes.
    pub(crate) const fn of(outcome: ExistenceOutcome) -> Self {
        match outcome {
            ExistenceOutcome::Bind(kind) => Self::Set(Existence::Bound(kind)),
            ExistenceOutcome::Unbind => Self::Set(Existence::Unbound),
            ExistenceOutcome::Preserve => Self::PRESERVE,
            ExistenceOutcome::MayBind(kind) => Self::Join(Existence::Bound(kind)),
        }
    }

    /// The fact afterwards on a place that held `prior`.
    pub(crate) const fn apply(self, prior: Existence) -> Existence {
        match self {
            Self::Set(fact) => fact,
            Self::Join(fact) => prior.join(fact),
        }
    }

    /// `self` followed by `next` on the same place, as one step.
    pub(crate) const fn then(self, next: Self) -> Self {
        match (self, next) {
            (_, Self::Set(fact)) => Self::Set(fact),
            (Self::Set(first), Self::Join(fact)) => Self::Set(first.join(fact)),
            (Self::Join(first), Self::Join(fact)) => Self::Join(first.join(fact)),
        }
    }

    /// The step two members of a finite input take together: the fact
    /// afterwards is the join of each member's.
    pub(crate) const fn join(self, other: Self) -> Self {
        match (self, other) {
            (Self::Set(left), Self::Set(right)) => Self::Set(left.join(right)),
            (Self::Set(left) | Self::Join(left), Self::Join(right))
            | (Self::Join(left), Self::Set(right)) => Self::Join(left.join(right)),
        }
    }
}

/// One definition's answer: the lattice value the statement leaves in it,
/// and the folded type the evaluation that produced the value states.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DefAnswer {
    /// The definition.
    pub(crate) key: ValueKey,
    /// Its lattice value.
    pub(crate) value: LatticeValue,
    /// Its folded type, when the evaluation states one.
    pub(crate) folded: Option<FoldedType>,
    /// Whether an evaluated outcome's stores name the definition's place,
    /// so the value is what the place holds afterwards. A definition the
    /// SSA could only fan out as a may-write — an element of an array a
    /// whole-array writer names — takes such a value as it stands rather
    /// than joining it with the prior version.
    pub(crate) stated: bool,
    /// Whether every store the outcome makes to the definition's place is
    /// a `Preserve`: the command left the place untouched, so the
    /// definition is the prior version's value and existence
    /// ([`crate::sccp::SccpResult::preserved`]).
    pub(crate) preserved: bool,
    /// What the statement does to the definition's place's existence.
    pub(crate) existence: ExistenceStep,
}

impl DefAnswer {
    /// A definition with `value`, no folded type, no store naming it, and
    /// the generic existence transfer, which widens.
    pub(crate) const fn untyped(key: ValueKey, value: LatticeValue) -> Self {
        Self {
            key,
            value,
            folded: None,
            stated: false,
            preserved: false,
            existence: ExistenceStep::UNKNOWN,
        }
    }

    /// The join of two members' answers for one definition: the values
    /// join as members do ([`join_members`]), the folded types keep what
    /// both state ([`FoldedType::join`]), and the place is stated only
    /// when both state it.
    fn join(self, other: &Self) -> Self {
        let folded = match (&self.folded, &other.folded) {
            (Some(left), Some(right)) => left.join(right),
            _ => None,
        };
        Self {
            key: self.key,
            value: join_members(&self.value, &other.value),
            folded,
            stated: self.stated && other.stated,
            preserved: self.preserved && other.preserved,
            existence: self.existence.join(other.existence),
        }
    }
}

/// Each word's literal text, `None` for a word that substitutes.
fn literal_words<'w>(cooked: &'w [ArgWord<'_>]) -> Vec<Option<&'w str>> {
    cooked
        .iter()
        .map(|word| (word.kind == InvocationWordKind::Literal).then_some(word.text.as_ref()))
        .collect()
}

/// Every definition in `defs` holding `value` and waiting on its existence:
/// a call whose callee does not complete normally, or whose places' facts
/// the solver has not reached.
fn waiting(defs: &[(String, ValueKey)], value: &LatticeValue) -> Vec<DefAnswer> {
    defs.iter()
        .map(|(_, key)| DefAnswer {
            existence: ExistenceStep::PENDING,
            ..DefAnswer::untyped(*key, value.clone())
        })
        .collect()
}

/// Bounds stating only a binding of `kind`.
const fn bound_to(kind: BindingKind) -> FactBounds {
    FactBounds {
        existence: Existence::Bound(kind),
        intrep: None,
        shape: None,
        segments: None,
        taint: None,
    }
}

/// Every definition in `defs` widened: the answer for a statement whose
/// evaluation declined.
fn widened(defs: &[(String, ValueKey)]) -> Vec<DefAnswer> {
    defs.iter()
        .map(|(_, key)| DefAnswer::untyped(*key, LatticeValue::Overdefined))
        .collect()
}

/// Whether a completion domain admits the normal completion.
fn completes_normally(domain: tcl_registry::completion::CompletionCodeDomain) -> bool {
    match domain {
        tcl_registry::completion::CompletionCodeDomain::Exact(codes) => {
            codes.contains(&tcl_registry::completion::CompletionCode::Ok)
        }
        tcl_registry::completion::CompletionCodeDomain::Any => true,
    }
}

/// The statement's definitions, each with the variable name it defines.
pub(crate) fn named_defs(stmt_ssa: &SsaStatement, ssa: &SsaFunction) -> Vec<(String, ValueKey)> {
    let mut defs: Vec<(String, ValueKey)> = stmt_ssa
        .defs
        .iter()
        .map(|(&sym, &ver)| (ssa.var_name(sym).to_owned(), (sym, ver)))
        .collect();
    defs.sort_unstable_by(|(left, _), (right, _)| left.cmp(right));
    defs
}

/// Two members' values for one definition, joined: a widened member widens
/// the definition and a pending one keeps it pending — a preserved prior
/// the solver has not reached is never taken for the other member's value
/// — and otherwise the definition holds the union of their constants.
fn join_members(left: &LatticeValue, right: &LatticeValue) -> LatticeValue {
    match (left, right) {
        (LatticeValue::Overdefined, _) | (_, LatticeValue::Overdefined) => {
            LatticeValue::Overdefined
        }
        (LatticeValue::Unknown, _) | (_, LatticeValue::Unknown) => LatticeValue::Unknown,
        _ => crate::sccp::join(left, right),
    }
}

/// A fact view's lattice projection: an exact value is its constant, a
/// finite set its constants, a pending input unknown, and anything else
/// widened.
fn fact_to_lattice(view: &FactView) -> LatticeValue {
    match view {
        FactView::Pending => LatticeValue::Unknown,
        FactView::Exact(value, _) => exact_to_lattice(value),
        FactView::Finite(members, _) => members
            .iter()
            .map(exact_to_lattice)
            .reduce(|left, right| join_members(&left, &right))
            .unwrap_or(LatticeValue::Overdefined),
        FactView::Domain(_) | FactView::Top(_) => LatticeValue::Overdefined,
    }
}

impl<'a> LatticeDriver<'a> {
    /// The driver for one SCCP run over `trace`'s registry and facts.
    pub(crate) fn new(
        trace: TraceInputs<'a>,
        folds: Option<BuiltinFoldInputs<'a>>,
        policy: FoldPolicy,
        escaping: &HashSet<String>,
    ) -> Self {
        let registry = trace.registry;
        let profile = trace
            .source_metadata_input
            .map(crate::analyser::ResolvedAnalysisInput::unit_profile)
            .or_else(|| registry.profile());
        let metadata_context = trace.source_metadata_input.map_or(
            Some(crate::registry_invocation::InvocationMetadataSelection::Standalone),
            |input| {
                crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
                    registry, input,
                )
                .map(crate::registry_invocation::InvocationMetadataSelection::Retained)
            },
        );
        let lexer_config = trace.source_metadata_input.map_or_else(
            || LexerConfig::for_profile(profile),
            crate::analyser::ResolvedAnalysisInput::lexer_config,
        );
        let key = trace.analysis_context;
        let context = AnalysisContext {
            registry_generation: key.map_or(0, |k| k.registry_generation),
            overlay_generation: key.and_then(|k| k.overlay_generation),
            bindings: key.map_or_default(|k| k.bindings.binding_evidence()),
            namespace: "::".to_owned(),
            profile,
            grammar: lexer_config.grammar_over(
                profile.map_or_else(tcl_dialect::LexerGrammar::default, |p| p.grammar),
            ),
            traced_variables: trace.traced_variables.clone(),
            has_dynamic_variable_trace: trace.has_dynamic_variable_trace,
            precision_written: folds.is_some_and(|f| f.mutations.may_set_precision()),
            escaping: escaping.iter().cloned().collect(),
            seeds_revision: 0,
            evaluator_revision: key.map_or(0, |k| k.evaluator_revision),
            tier: key.map_or(AnalysisTier::Deep, |k| k.tier),
            script_encoding: trace
                .source_metadata_input
                .map_or_default(crate::analyser::ResolvedAnalysisInput::script_encoding),
        };
        let mut request = request_budget();
        let iteration = request.iteration();
        let typed_assignment = folds.is_none_or(|f| {
            typed_node_commands(registry, LoweringHookId::Set)
                .iter()
                .all(|name| f.mutations.observed_binding_is_the_builtin(name))
        });
        Self {
            registry,
            metadata_context,
            source_metadata_input: trace.source_metadata_input,
            folds,
            policy,
            expression_preparations: HashMap::new(),
            context,
            lexer_config,
            typed_assignment: typed_assignment && metadata_context.is_some(),
            nesting: Cell::new(0),
            explaining: Cell::new(None),
            throwing: Cell::new(false),
            word_error: Cell::new(false),
            explanations: RefCell::new(BTreeMap::new()),
            folded: RefCell::new(HashMap::new()),
            preserved: RefCell::new(HashMap::new()),
            raised: RefCell::new(HashSet::new()),
            reads_exits: false,
            uncaught: RefCell::new(HashSet::new()),
            undecided: Cell::new(false),
            tally: Cell::new(RouteTally::default()),
            request: RefCell::new(request),
            iteration: RefCell::new(iteration),
            existence: RefCell::new(None),
            existence_places: RefCell::new(HashMap::new()),
            existence_external: RefCell::new(Vec::new()),
            existence_initial_global: trace.existence.is_some_and(|entry| entry.initial_global),
            catch_ends: RefCell::new(HashMap::new()),
            body_entry: RefCell::new(None),
            module: None,
            level: crate::sccp::ModuleLevel::Places,
            function: "::",
            body_id: None,
            reads_module: Cell::new(false),
        }
    }

    /// Retain the CFG's original availability mode. A missing actual input
    /// cannot acquire detached catalogue compatibility from `TraceInputs::None`.
    pub(crate) fn with_cfg_metadata(
        mut self,
        owner: &'a crate::registry_invocation::OwnedInvocationMetadataContext,
    ) -> Self {
        self.metadata_context = owner.metadata_context(self.registry);
        if let Some(input) = owner.source_analysis_input() {
            let agrees = self.source_metadata_input.is_some_and(|trace| {
                std::ptr::eq(
                    trace.borrowed_context_registry(),
                    input.borrowed_context_registry(),
                ) && trace.unit_profile().cache_key() == input.unit_profile().cache_key()
                    && trace.analyser_profile().cache_key() == input.analyser_profile().cache_key()
                    && trace.lexer_config().normalized() == input.lexer_config().normalized()
                    && trace.vendor_source_policy() == input.vendor_source_policy()
            });
            if !agrees {
                self.metadata_context = None;
            }
            self.context.profile = Some(input.unit_profile());
            self.context.grammar = input
                .lexer_config()
                .grammar_over(input.unit_profile().grammar);
            self.lexer_config = input.lexer_config();
        } else if owner.is_standalone() {
            if self.source_metadata_input.is_some() {
                self.metadata_context = None;
            }
        } else {
            // Availability-only or withdrawn ownership supplies no source
            // grammar/profile from a nominal registry label.
            self.context.profile = None;
            // dialect-drift-ok: withdrawn ownership claims no source grammar;
            // the neutral one is the explicit absence of a document's
            self.context.grammar = tcl_dialect::LexerGrammar::default();
        }
        self.typed_assignment &= self.metadata_context.is_some();
        self
    }

    /// The grammar the run reads its document's text under.
    pub(crate) const fn grammar(&self) -> &tcl_dialect::LexerGrammar {
        &self.context.grammar
    }

    /// Say which function the run analyses and what it reads of the module
    /// ([`crate::sccp::ModuleRun`]). The module's summary revision rides the
    /// context's seeds revision.
    pub(crate) fn in_module(
        mut self,
        cfg: &'a crate::cfg::Function,
        run: crate::sccp::ModuleRun<'a>,
    ) -> Self {
        self.function = &cfg.name;
        if let Some(context) = self.policy.preparation_context() {
            for (block_id, block) in &cfg.blocks {
                for (index, statement) in block.statements.iter().enumerate() {
                    let Statement::AssignExpr { expr_base, .. } = statement else {
                        continue;
                    };
                    let proof = crate::math_function_binding::ExpressionMathBindings::for_origin(
                        &cfg.implicit_math_invocations,
                        cfg.statement_sources
                            .get(&(*block_id, index))
                            .and_then(Option::as_deref),
                        *expr_base,
                    )
                    .with_preparations(&cfg.expression_preparations)
                    .preparation_for_context(&context);
                    self.expression_preparations
                        .entry(statement.span())
                        .and_modify(|existing| {
                            if *existing != proof {
                                *existing = None;
                            }
                        })
                        .or_insert(proof);
                }
            }
        }
        self.body_id = run
            .analysis_body
            .or_else(|| run.procedures.and_then(|module| module.body_for_cfg(cfg)));
        self.module = run.procedures;
        self.level = run.level;
        self.reads_exits = run.reads_exits;
        self.context.seeds_revision = run
            .procedures
            .map_or(0, crate::interprocedural::ModuleProcedures::revision);
        self
    }

    /// The trust a procedure binding is read under: the rewrite stance's
    /// whole-module trust, or the shared lattice's observed bindings.
    fn procedure_trust(&self) -> FoldTrust {
        self.folds
            .map_or(FoldTrust::WholeModule, |folds| folds.trust)
    }

    /// The default `info default` reads for `procedure`'s `parameter`
    /// ([`crate::interprocedural::ModuleProcedures::parameter_default`]):
    /// unknown in a run whose caller holds no module view, which records
    /// that it would have read one.
    pub(crate) fn parameter_default_at(
        &self,
        original: Option<&CommandTokens>,
        procedure: &str,
        parameter: &str,
    ) -> ParameterDefault {
        self.reads_module.set(true);
        self.module.map_or(ParameterDefault::Unknown, |module| {
            module.parameter_default(original, (procedure, parameter), self.procedure_trust())
        })
    }

    /// Whether a typed assignment (`AssignConst`, `AssignValue`,
    /// `AssignExpr`) still means `set`. A module that shadows or renames
    /// `set` turns `set s hello` into a call to something else, which need
    /// not write `s` at all: `proc set {name value} {return ZZZ}; set s
    /// hello; append s world; puts $s` prints `world` in every release.
    /// Without a trust fact the typed node's own lowering stands.
    pub(crate) const fn typed_assignment_trusted(&self) -> bool {
        self.typed_assignment
    }

    /// A single-token literal word's value under the document's grammar —
    /// [`literal_token_value`] for the typed statements, which carry their
    /// word's text rather than its value.
    pub(crate) fn literal_value<'t>(&self, text: &'t str, kind: TokenType) -> Option<Cow<'t, str>> {
        literal_token_value(text, kind, &self.lexer_config)
    }

    /// A driver for a statement evaluated outside a run: the fold inputs'
    /// registry when there are any, else the process-wide default.
    pub(crate) fn detached(folds: Option<BuiltinFoldInputs<'a>>, policy: FoldPolicy) -> Self {
        let registry = match folds {
            Some(f) => f.registry,
            None => tcl_registry::default_registry(),
        };
        Self::outside_a_run(registry, folds, policy)
    }

    /// A driver for a loop enumeration outside a run, over `registry` with no
    /// fold inputs, so it takes the registry's table for every head
    /// ([`Self::enumeration_trusts`]).
    pub(crate) fn over_registry(registry: &'a CommandRegistry, policy: FoldPolicy) -> Self {
        Self::outside_a_run(registry, None, policy)
    }

    /// A driver with no run around it: no traced place, no escaping one, no
    /// deferred write and no existence rung.
    fn outside_a_run(
        registry: &'a CommandRegistry,
        folds: Option<BuiltinFoldInputs<'a>>,
        policy: FoldPolicy,
    ) -> Self {
        Self::new(
            TraceInputs {
                source_metadata_input: folds.and_then(|inputs| inputs.source_metadata_input),
                registry,
                traced_variables: &EMPTY_NAMES,
                has_dynamic_variable_trace: false,
                deferred_writes: &crate::ir::NO_DEFERRED_WRITES,
                analysis_context: None,
                existence: None,
            },
            folds,
            policy,
            &HashSet::new(),
        )
    }

    /// Binding validity for `head`, under the stance the caller states
    /// ([`FoldTrust`]): a fold that becomes a source rewrite asks the whole
    /// module ([`crate::command_binding::ModuleCommandMutations::trusts`],
    /// the unbounded top included); the shared per-unit lattice asks only
    /// the module's own observed bindings
    /// ([`crate::command_binding::ModuleCommandMutations::observed_binding_is_the_builtin`]),
    /// so one unresolved head does not cost a file every constant. Without
    /// the fact there is no evidence the name still denotes its registry
    /// command, and every route declines: a lattice that trusted every
    /// binding answered `3` for `[llength {a b c}]` where the module's own
    /// `proc llength` returns 99 (#2164).
    pub(crate) fn trusted(&self, head: &str) -> bool {
        self.folds.is_some_and(|f| match f.trust {
            FoldTrust::WholeModule => f.mutations.trusts(head),
            FoldTrust::ObservedBindings => f.mutations.observed_binding_is_the_builtin(head),
        })
    }

    /// The first command binding a declared implementation's answer rests on
    /// (`depends {… binding NAME}`) that the run's trust stance does not
    /// hold to be the command the declaration names: the body runs the
    /// builtin, and a module that renames or shadows it calls something
    /// else. A run with no trust stance takes the registry's table, as an
    /// expression's head is taken.
    fn unheld_binding<'c>(
        &self,
        capability: &'c tcl_registry::value_transfer::EvaluatorCapability,
    ) -> Option<&'c str> {
        capability
            .depends
            .iter()
            .find_map(|dependency| match dependency {
                tcl_registry::value_transfer::ContextDependency::Binding(binding)
                    if self.folds.is_some() && !self.trusted(&binding.name) =>
                {
                    Some(binding.name.as_str())
                }
                _ => None,
            })
    }

    /// Whether a declared implementation rests on a binding the run does
    /// not hold ([`Self::unheld_binding`]); the explanation names it.
    fn explains_an_unheld_binding(
        &self,
        head: &str,
        route: EvalRoute,
        capability: &tcl_registry::value_transfer::EvaluatorCapability,
    ) -> bool {
        let Some(name) = self.unheld_binding(capability) else {
            return false;
        };
        self.explain(
            head,
            Some(route),
            format!("declined: the binding `{name}` it depends on does not hold"),
        );
        true
    }

    /// Whether a caller proved the procedure the run evaluates pure before
    /// evaluating it with constant arguments
    /// ([`BuiltinFoldInputs::proven_pure_parameters`]).
    pub(crate) fn proven_pure_parameters(&self) -> bool {
        self.folds.is_some_and(|f| f.proven_pure_parameters)
    }

    /// Resolve through the retained availability generation. Only explicitly
    /// standalone runs use the registry's catalogue surface.
    fn resolve<'w>(
        &self,
        head: &'w str,
        words: &'w [InvocationWord<'w>],
    ) -> Option<ResolvedInvocation<'a, 'w>>
    where
        'a: 'w,
    {
        match self.metadata_context?.context() {
            Some(context) => {
                tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
                    self.registry,
                    Some(context.context()),
                    InvocationWords::structured(InvocationWord::Literal(head), words),
                    tcl_dialect::model::InvocationRealm::RuleLoader,
                )
            }
            None => self.registry.resolve_structured_invocation(
                InvocationWords::structured(InvocationWord::Literal(head), words),
                self.registry.own_surface_query(),
            ),
        }
        .resolved()
    }
}

static EMPTY_NAMES: BTreeSet<String> = BTreeSet::new();

/// The same positioned word evaluation reads either executable facts or the
/// separately authenticated diagnostic contents. Only executable facts carry
/// a representation projection.
#[derive(Clone, Copy)]
pub(crate) struct ProvenReadFacts<'a> {
    values: &'a HashMap<ValueKey, LatticeValue>,
    folded_types: Option<&'a HashMap<ValueKey, FoldedType>>,
    reached: &'a HashSet<crate::cfg::BlockId>,
    /// The registry whose const-folds prove a whole-word command
    /// substitution; `None` proves none.
    folds: Option<&'a CommandRegistry>,
}

/// One host evaluated under `LocalWrites`.
struct Ordered {
    /// The evaluation's answer, lifted over one finite input.
    answer: LiftedAnswer,
    /// Each evaluated outcome's writes, in execution order and resolved to
    /// places.
    writes: Vec<Vec<(PlaceRef, StoreOutcome)>>,
    /// The folded type the result states, when it is a whole word's.
    folded: Option<FoldedType>,
}

/// One `[…]` script run on its declared route.
struct ScriptRun {
    /// The command's head as the script spells it.
    head: String,
    /// The declared route, when the head is still the builtin.
    route: Option<EvalRoute>,
    /// The route's answer, lifted over one finite input.
    answer: LiftedAnswer,
    /// Each evaluated outcome's writes, in execution order and resolved to
    /// their places: its substitutions' first, then its own stores. Made
    /// only for a run under [`NestedPolicy::LocalWrites`], one entry per
    /// outcome of `answer`.
    writes: Vec<Vec<(PlaceRef, StoreOutcome)>>,
    /// The binding the answer rests on.
    binding: BindingIdentity,
    /// Whether the head no longer denotes its registry command.
    rebound: bool,
    /// Whether the head names a procedure of the module, run as its transfer
    /// summary says ([`LatticeDriver::procedure_run`]).
    procedure: bool,
}

/// The binding `head` resolves to in the global namespace, expected to be
/// the registry's `identity`.
fn binding_of(head: &str, identity: &str) -> BindingIdentity {
    BindingIdentity {
        resolution_namespace: String::new(),
        name: head.to_owned(),
        identity: identity.to_owned(),
    }
}

/// Record `binding` in `evidence` once.
fn record_binding(evidence: &mut DependencyEvidence, binding: BindingIdentity) {
    if !evidence.bindings.contains(&binding) {
        evidence.bindings.push(binding);
    }
}

/// An outcome's result as a lattice value, when it is exact.
fn result_of(outcome: &InvocationOutcome) -> Option<LatticeValue> {
    match &outcome.result {
        ExactValueOrUnavailable::Exact(value) => Some(exact_to_lattice(value)),
        ExactValueOrUnavailable::Unavailable(_) => None,
    }
}

/// The expression route as the lift runs it: the shared engine over the
/// analysis services ([`evaluate_expression`]), so a finite input the
/// expression reads is pinned per member like any route's operand.
struct ExpressionEvaluation<'e> {
    /// The expression.
    expression: Expression<'e>,
    /// The value semantics the engine evaluates under.
    policy: FoldPolicy,
    /// Which nested operations the evaluation admits: only effect-free
    /// ones, or writes to places its state can own.
    nested: NestedPolicy,
    /// The `expr` binding the answer rests on; a branch condition is read
    /// by its command's own expression parser and rests on none.
    head: Option<BindingIdentity>,
}

/// One argument of an invocation as the resolver and an evaluator read it:
/// a literal word's value, cooked, or a substituted word's raw spelling,
/// which the lattice reads by name.
struct ArgWord<'t> {
    /// The literal's value, or the substituted word's spelling.
    text: Cow<'t, str>,
    /// What the source proves about the word.
    kind: InvocationWordKind,
    /// How the word substitutes.
    source: OperandSource,
}

/// The analyser's read-only inputs over the SCCP lattice at one statement.
struct LatticeInputs<'a, S1, S2> {
    original: Option<CommandTokens>,
    written_arguments: Vec<Option<usize>>,
    expression_base: Option<u32>,
    expression_source: RefCell<Option<crate::interprocedural::OriginalSummaryExpression>>,
    nested_source: RefCell<Option<crate::interprocedural::OriginalSummaryScript>>,
    driver: &'a LatticeDriver<'a>,
    view: ResolvedInvocationView<'a>,
    uses: &'a HashMap<Symbol, Version, S1>,
    values: &'a HashMap<ValueKey, LatticeValue, S2>,
    ssa: &'a SsaFunction,
    /// The operation the uses are read at, where the SSA binds a spelling
    /// per operation ([`SsaFunction::var_symbol_at`]).
    point: Option<(crate::cfg::BlockId, usize)>,
    /// How each operand's word substitutes, in operand order.
    sources: Vec<OperandSource>,
    /// The writes the enclosing evaluations have made before this
    /// invocation runs, in order: a read consults them before the lattice
    /// at the program point (the ordered evaluation state's rule, § `expr`).
    prior_writes: Vec<(PlaceRef, StoreOutcome)>,
    /// How the operand words that substitute are evaluated.
    words: Words,
}

/// The inputs an invocation or an expression reads from a loop
/// enumeration's state: an operand is a literal word, or a word substituted
/// over the state — a variable exact where the state holds a value, unbound
/// where it holds none, and unknown where the state does not hold it, and a
/// `[…]` script one command run over the state by its route, under the
/// effect-free policy; a math function is the one the run's driver resolves,
/// so one the module rebinds declines.
struct StateInputs<'a> {
    driver: &'a LatticeDriver<'a>,
    view: ResolvedInvocationView<'a>,
    state: &'a crate::static_loops::LoopState,
}

/// The element `key` of the array `base` names: the place an element write
/// lands on. An element of an element is no place — the command raises on
/// it — so it declines.
fn element_place(base: &PlaceRef, key: &str) -> Result<PlaceRef, DeclineReason> {
    if base.is_element() {
        return Err(DeclineReason::Unsupported);
    }
    Ok(PlaceRef {
        name: format!("{}({key})", base.name),
        kind: PlaceKind::Element {
            base: base.name.clone(),
            key: key.to_owned(),
        },
    })
}

/// A place for a normalised name.
pub(crate) fn place_named(name: &str) -> PlaceRef {
    let kind = crate::naming::split_element_ref(name).map_or(PlaceKind::Scalar, |(base, key)| {
        PlaceKind::Element {
            base: base.to_owned(),
            key: key.to_owned(),
        }
    });
    PlaceRef {
        name: name.to_owned(),
        kind,
    }
}

/// Split a command-substitution body into `(head_word, rest)`. `rest` is
/// `None` if the body is a single word, otherwise the remaining text with
/// the leading whitespace stripped.
pub(crate) fn split_head(text: &str) -> (&str, Option<&str>) {
    let trimmed = text.trim_start();
    let end = trimmed.find(char::is_whitespace).unwrap_or(trimmed.len());
    let head = &trimmed[..end];
    if end >= trimmed.len() {
        return (head, None);
    }
    let rest = trimmed[end..].trim_start();
    if rest.is_empty() {
        (head, None)
    } else {
        (head, Some(rest))
    }
}

/// Resolve `name` to the textual form of its lattice constant at this
/// statement's use version, or `None` when it is not a single `Const` —
/// the variable lookup the registry const-fold engine runs under.
pub(crate) fn lattice_const_text<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
    name: &str,
    uses: &HashMap<Symbol, Version, S1>,
    values: &HashMap<ValueKey, LatticeValue, S2>,
    ssa: &SsaFunction,
) -> Option<String> {
    let sym = ssa.var_symbol(name)?;
    let ver = uses.get(&sym)?;
    match values.get(&(sym, *ver))? {
        LatticeValue::Const(c) => {
            Some(String::from_utf8_lossy(&const_to_exact(c).bytes).into_owned())
        }
        _ => None,
    }
}

/// `text` over a frame where `x` holds 1 and `::g` is a global the
/// function names, evaluated under `nested` by a detached driver that
/// trusts every builtin: the nested service's answer, as the tests of
/// the evaluation read it.
#[cfg(test)]
pub(crate) fn evaluate_over_x(text: &str, nested: NestedPolicy) -> LiftedAnswer {
    // One named release: 8.4 to 9.1 agree on every expression these tests
    // evaluate, and without one the numeral grammar is ambiguous.
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").expect("tcl8.6");
    let registry = CommandRegistry::build_default();
    let mutations = crate::command_binding::ModuleCommandMutations::default();
    let driver = LatticeDriver::detached(
        Some(BuiltinFoldInputs {
            source_metadata_input: None,
            registry: &registry,
            mutations: &mutations,
            dialect: None,
            defining_class: None,
            registry_engine: false,
            trust: crate::sccp::FoldTrust::ObservedBindings,
            proven_pure_parameters: false,
        }),
        FoldPolicy::for_profile(None, Some(profile)),
    );
    let node = crate::expr_parser::parse_expr_for_profile(text, None);
    let expression = ExpressionEvaluation {
        expression: Expression::Parsed(&node),
        policy: FoldPolicy::for_profile(None, Some(profile)),
        nested,
        head: None,
    };
    let mut ssa = SsaFunction::trivial("::p", crate::cfg::BlockId(0), vec!["entry".into()]);
    let x = ssa.intern_var("x");
    let g = ssa.intern_var("::g");
    let uses: HashMap<Symbol, Version> = HashMap::from([(x, 1), (g, 1)]);
    let values: HashMap<ValueKey, LatticeValue> = HashMap::from([
        ((x, 1), LatticeValue::Const(ConstValue::Int(1))),
        ((g, 1), LatticeValue::Const(ConstValue::Int(5))),
    ]);
    driver.evaluate_expression_at(&expression, None, &uses, &values, &ssa)
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod original_child_script_purpose_tests;
