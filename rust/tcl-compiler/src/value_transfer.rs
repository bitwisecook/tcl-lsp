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
//! ([`NativeEvalId::owner`](tcl_registry::value_transfer::NativeEvalId::owner));
//! the compiler's transitional handlers are
//! gone. The expression route is run here by
//! construction: the registry assembles the argument words
//! ([`ExpressionRoute::assemble`]) and the shared engine evaluates the
//! expression over this module's lattice services
//! ([`crate::tcl_expr_eval::ExprServices`]).

use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use rustc_hash::FxHashSet;
use tcl_lexer::{LexerConfig, Span, TokenType};
use tcl_registry::hooks::LoweringHookId;
use tcl_registry::value_transfer::builtins::ExpressionRoute;
use tcl_registry::value_transfer::{
    AnalysisContext, AnalysisInputs, AnalysisTier, BindingIdentity, BodyRegion, Budget,
    BudgetLimit, CommandSemantics, CompletionOutcome, DeclineReason, DependencyEvidence,
    EvalAnswer, EvalRoute, EvaluationState, EvaluatorOwner, ExactValue, ExactValueOrUnavailable,
    ExistenceOutcome, FactDomain, FactView, InvocationLayout, InvocationOutcome, IterableKind,
    LanguageProfileId, LiftedAnswer, NestedPolicy, NumericValue, OperandId, OperandView, PlaceKind,
    PlaceRef, PlanAnswer, RepresentationEvidence, ResolvedInvocationView, RouteIdentity,
    StoreOutcome, TargetId, TransferAnswer, TypeFacts, ValueIdentity, ValueShape, WordPart,
    WordStructure, evaluate_lifted, validate_outcome,
};
use tcl_registry::{
    ArgRole, CommandRegistry, InvocationWord, InvocationWordKind, InvocationWords,
    ResolvedInvocation, SemanticOperationId, TclType,
};

use crate::analyses::{ConstValue, LatticeValue, MAX_CONSTSET_SIZE};
use crate::cfg::Function as CfgFunction;
use crate::codegen::helpers::split_list_values;
use crate::command_binding::CommandTrustSnapshot;
use crate::expr_ast::ExprNode;
use crate::ir::{CommandTokens, Statement};
use crate::sccp::{
    BuiltinFoldInputs, FoldTrust, TraceInputs, extract_foreach_elements,
    resolve_foreach_list_via_lattice,
};
use crate::ssa::{SsaFunction, SsaStatement, Symbol, ValueKey, Version};
use crate::tcl_expr_eval::{ExprAnswer, ExprServices, FoldPolicy, evaluate_expression};
use tcl_syntax::word_rules::WordValueRules;

/// The analysis context's hashable identity, as a per-function memo key
/// carries it: the facts that can change an answer and that the key's
/// other fields (body, dialect, grammar, traces, seeds) do not already
/// cover. Built once per module and shared by every function in it, so a
/// `rename` anywhere in the module re-keys every function's lattice — the
/// sensitivity the design asks for.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AnalysisContextKey {
    /// The generation of the registry the module resolved against
    /// ([`CommandRegistry::generation`]): the exact command surface, so two
    /// builds against different registries never share a lattice.
    pub registry_generation: u64,
    /// The workspace pack overlay that registry carries
    /// ([`CommandRegistry::overlay_generation`]), which the memoised
    /// per-procedure lattice resolves its own registry by.
    pub overlay_generation: Option<u64>,
    /// The module's command-binding evidence.
    pub bindings: CommandTrustSnapshot,
    /// The precision tier the request runs at.
    pub tier: AnalysisTier,
    /// The generation of the building thread's evaluator host
    /// ([`tcl_registry::pack_hooks::evaluator_generation`]): which host
    /// serves the declared implementations, and in what health, so a
    /// host-present and a host-absent worker never share a lattice.
    pub evaluator_revision: u64,
}

impl AnalysisContextKey {
    /// The key for a module whose command mutations were scanned, resolved
    /// against `registry`, under the building thread's evaluator
    /// generation.
    #[must_use]
    pub fn for_module(
        mutations: &crate::command_binding::ModuleCommandMutations,
        registry: &CommandRegistry,
    ) -> Self {
        Self {
            registry_generation: registry.generation(),
            overlay_generation: registry.overlay_generation(),
            bindings: mutations.snapshot(),
            tier: AnalysisTier::Deep,
            evaluator_revision: u64::from(tcl_registry::pack_hooks::evaluator_generation().0),
        }
    }

    /// The key for a consumer with no module view: no mutations observed,
    /// against the process-wide default registry.
    #[must_use]
    pub fn detached() -> Self {
        Self::for_module(
            &crate::command_binding::ModuleCommandMutations::default(),
            tcl_registry::default_registry(),
        )
    }
}

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

/// The semantic type, shape and representation evidence of one value, from
/// the evaluation that produced it (`docs/design/compiler/value-transfers.md`
/// § *Exact values, types, and representation*): three facts, kept apart
/// from the exact value in [`crate::sccp::SccpResult::values`] and from one
/// another. A computed byte array's string is exact while its
/// representation is the byte array the route built, so a consumer asking
/// whether a use converts reads [`Self::representation`], never the type
/// alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoldedType {
    /// The internal representation the evaluation's type facts state.
    pub intrep: Option<TclType>,
    /// The value's shape, when proven: a list's length, a dict's keys.
    pub shape: Option<ValueShape>,
    /// How the route constructed the value, when it says.
    pub representation: RepresentationEvidence,
}

impl FoldedType {
    /// The folded type of an outcome's result: its type facts' result type
    /// and the result value's representation evidence.
    #[must_use]
    pub fn of_result(outcome: &InvocationOutcome) -> Option<Self> {
        let representation = match &outcome.result {
            ExactValueOrUnavailable::Exact(value) => value.representation,
            ExactValueOrUnavailable::Unavailable(_) => RepresentationEvidence::Unknown,
        };
        Self::informative(outcome.types.result, None, representation)
    }

    /// The folded type a place holds after `stores`, the outcome's stores to
    /// it in execution order: a write's stated type and shape with the
    /// written value's representation; a may-write's bounds, its
    /// representation unknown; nothing after an unbind. A preserve keeps
    /// what an earlier store left and states nothing of the prior value.
    #[must_use]
    pub fn after_stores<'s>(
        outcome: &InvocationOutcome,
        stores: impl IntoIterator<Item = &'s StoreOutcome>,
    ) -> Option<Self> {
        let stated = |target: TargetId| {
            let intrep = outcome
                .types
                .per_target
                .iter()
                .rev()
                .find_map(|(at, ty)| (*at == target).then_some(*ty));
            let shape = outcome
                .types
                .shapes
                .iter()
                .rev()
                .find_map(|(at, shape)| (*at == target).then(|| shape.clone()));
            (intrep, shape)
        };
        let mut held: Option<Self> = None;
        for store in stores {
            held = match store {
                StoreOutcome::Write { target, value } => {
                    let (intrep, shape) = stated(*target);
                    Self::informative(intrep, shape, value.representation)
                }
                StoreOutcome::Preserve { .. } => held,
                StoreOutcome::MayWrite { facts, .. } => Self::informative(
                    facts.intrep,
                    facts.shape.clone(),
                    RepresentationEvidence::Unknown,
                ),
                StoreOutcome::Unbind { .. } => None,
            };
        }
        held
    }

    /// The folded type, when any of its three facts says something.
    fn informative(
        intrep: Option<TclType>,
        shape: Option<ValueShape>,
        representation: RepresentationEvidence,
    ) -> Option<Self> {
        (intrep.is_some() || shape.is_some() || representation != RepresentationEvidence::Unknown)
            .then_some(Self {
                intrep,
                shape,
                representation,
            })
    }

    /// The join of two folded types: each fact survives only where both
    /// state it alike, so identical strings built with different
    /// representations keep neither representation, and "the last written
    /// type wins" never happens.
    #[must_use]
    pub fn join(&self, other: &Self) -> Option<Self> {
        Self::informative(
            (self.intrep == other.intrep)
                .then_some(self.intrep)
                .flatten(),
            (self.shape == other.shape)
                .then(|| self.shape.clone())
                .flatten(),
            if self.representation == other.representation {
                self.representation
            } else {
                RepresentationEvidence::Unknown
            },
        )
    }

    /// The join of every member's folded type: `None` once any member has
    /// none, and for no members at all.
    #[must_use]
    pub fn join_all(members: impl IntoIterator<Item = Option<Self>>) -> Option<Self> {
        let mut members = members.into_iter();
        let first = members.next()??;
        members.try_fold(first, |joined, member| joined.join(&member?))
    }

    /// The internal representation the value holds when it is created: the
    /// one the route constructed, when it says. A computed string holds
    /// none (a pure string), so this is `None` for it too.
    #[must_use]
    pub fn constructed_intrep(&self) -> Option<TclType> {
        match self.representation {
            RepresentationEvidence::Constructed(TclType::String)
            | RepresentationEvidence::Unknown => None,
            RepresentationEvidence::Constructed(built) => Some(built),
        }
    }

    /// The Explorer's spelling: the stated type, with how the route built
    /// the value — `bytearray (constructed)`, `string (constructed as
    /// int)`, or `list` when only the type is stated.
    #[must_use]
    pub fn label(&self) -> String {
        let named = |ty: TclType| crate::shimmer::type_name(ty);
        match (self.intrep, self.representation) {
            (Some(ty), RepresentationEvidence::Constructed(built)) if ty == built => {
                format!("{} (constructed)", named(ty))
            }
            (Some(ty), RepresentationEvidence::Constructed(built)) => {
                format!("{} (constructed as {})", named(ty), named(built))
            }
            (None, RepresentationEvidence::Constructed(built)) => {
                format!("{} (constructed)", named(built))
            }
            (Some(ty), RepresentationEvidence::Unknown) => named(ty),
            (None, RepresentationEvidence::Unknown) => "?".to_owned(),
        }
    }
}

/// The driver's per-run state: the registry the unit resolves against, the
/// whole-module trust fact when the caller holds one, the fold policy, the
/// analysis context every answer is memoised under, and the route
/// explanations the run records for the statement it is evaluating.
pub(crate) struct LatticeDriver<'a> {
    registry: &'a CommandRegistry,
    folds: Option<BuiltinFoldInputs<'a>>,
    policy: FoldPolicy,
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
    /// The last explanation recorded per statement.
    explanations: RefCell<BTreeMap<(u32, u32), RouteExplanation>>,
    /// The folded type of each definition the last evaluation of its
    /// statement stated one for ([`crate::sccp::SccpResult::folded_types`]).
    folded: RefCell<HashMap<ValueKey, FoldedType>>,
    /// The run's route-entry counts so far.
    tally: Cell<RouteTally>,
    /// The run's request budget: every evaluation of this run charges
    /// through it, so a run whose evaluations spend it declines the rest.
    request: RefCell<Budget>,
    /// The current solver pass's share of the request.
    iteration: RefCell<Budget>,
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
}

impl DefAnswer {
    /// A definition with `value` and no folded type.
    pub(crate) const fn untyped(key: ValueKey, value: LatticeValue) -> Self {
        Self {
            key,
            value,
            folded: None,
        }
    }

    /// The join of two members' answers for one definition: the values
    /// join as members do ([`join_members`]) and the folded types keep
    /// what both state ([`FoldedType::join`]).
    fn join(self, other: &Self) -> Self {
        let folded = match (&self.folded, &other.folded) {
            (Some(left), Some(right)) => left.join(right),
            _ => None,
        };
        Self {
            key: self.key,
            value: join_members(&self.value, &other.value),
            folded,
        }
    }
}

/// Every definition in `defs` widened: the answer for a statement whose
/// evaluation declined.
fn widened(defs: &[(String, ValueKey)]) -> Vec<DefAnswer> {
    defs.iter()
        .map(|(_, key)| DefAnswer::untyped(*key, LatticeValue::Overdefined))
        .collect()
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

/// The value an outcome writes to `target`, when it writes one.
fn written_value(outcome: &InvocationOutcome, target: TargetId) -> Option<&ExactValue> {
    outcome.ordered_stores.iter().find_map(|store| match store {
        StoreOutcome::Write { target: t, value } if *t == target => Some(value),
        _ => None,
    })
}

/// One solver run's request budget: [`Budget::request`], or the smaller
/// one a test sets to watch a run exhaust it.
fn request_budget() -> Budget {
    #[cfg(test)]
    if let Some(work) = tests::REQUEST_WORK.with(Cell::get) {
        return Budget::request_of(work, Budget::REQUEST_RETAINED_BYTES);
    }
    Budget::request()
}

/// The route's spelling for an explanation.
fn route_label(route: Option<EvalRoute>) -> String {
    match route {
        None => "no semantics".to_owned(),
        Some(EvalRoute::Direct { id }) => match id.owner() {
            EvaluatorOwner::Registry => format!("direct {} (registry)", id.as_str()),
            EvaluatorOwner::Transitional { retires_in_slice } => format!(
                "direct {} (compiler, transitional until slice {retires_in_slice})",
                id.as_str()
            ),
        },
        Some(EvalRoute::Expression { language }) => format!("expression {}", language.as_str()),
        Some(EvalRoute::Implementation(capability)) => {
            format!("implementation {}", capability.identity.id)
        }
        Some(EvalRoute::None { reason }) => format!("none ({})", reason.as_str()),
    }
}

/// The decline's spelling for an explanation, payload included.
fn reason_label(reason: DeclineReason) -> String {
    match reason {
        DeclineReason::ReleaseAmbiguous(axis) => {
            format!("{}: {}", reason.as_str(), axis.as_str())
        }
        DeclineReason::NoRoute(why) => format!("{}: {}", reason.as_str(), why.as_str()),
        DeclineReason::Unavailable(tier) => format!("{}: {}", reason.as_str(), tier.as_str()),
        DeclineReason::Budget(limit) => format!("{}: {limit:?}", reason.as_str()),
        other => other.as_str().to_owned(),
    }
}

/// The lifted answer's spelling for an explanation.
fn answer_label(answer: &LiftedAnswer) -> String {
    match answer {
        LiftedAnswer::Pending => "pending".to_owned(),
        LiftedAnswer::Declined(reason) => format!("declined: {}", reason_label(*reason)),
        LiftedAnswer::Evaluated(outcomes) if outcomes.len() == 1 => "evaluated".to_owned(),
        LiftedAnswer::Evaluated(outcomes) => {
            format!("evaluated per member ({} members)", outcomes.len())
        }
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
        let profile = registry.profile();
        let key = trace.analysis_context;
        let context = AnalysisContext {
            registry_generation: key.map_or(0, |k| k.registry_generation),
            overlay_generation: key.and_then(|k| k.overlay_generation),
            bindings: key
                .map(|k| k.bindings.binding_evidence())
                .unwrap_or_default(),
            namespace: "::".to_owned(),
            profile,
            grammar: profile.map_or_else(tcl_dialect::LexerGrammar::default, |p| p.grammar),
            traced_variables: trace.traced_variables.clone(),
            has_dynamic_variable_trace: trace.has_dynamic_variable_trace,
            escaping: escaping.iter().cloned().collect(),
            seeds_revision: 0,
            evaluator_revision: key.map_or(0, |k| k.evaluator_revision),
            tier: key.map_or(AnalysisTier::Deep, |k| k.tier),
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
            folds,
            policy,
            context,
            lexer_config: LexerConfig::for_profile(profile),
            typed_assignment,
            nesting: Cell::new(0),
            explaining: Cell::new(None),
            explanations: RefCell::new(BTreeMap::new()),
            folded: RefCell::new(HashMap::new()),
            tally: Cell::new(RouteTally::default()),
            request: RefCell::new(request),
            iteration: RefCell::new(iteration),
        }
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

    /// The statement the solver is about to evaluate, so the answers below
    /// are recorded against it; `None` between statements.
    pub(crate) fn explaining(&self, span: Option<Span>) {
        self.explaining.set(span);
    }

    /// Record `command`'s route and `answer` for the statement being
    /// evaluated. The last pass over a statement wins, so the record is the
    /// fixed point's.
    fn explain(&self, command: &str, route: Option<EvalRoute>, answer: String) {
        let Some(span) = self.explaining.get() else {
            return;
        };
        self.explanations.borrow_mut().insert(
            (span.start(), span.end()),
            RouteExplanation {
                span,
                command: command.to_owned(),
                route: route_label(route),
                answer,
            },
        );
    }

    /// Every explanation the run recorded, in statement order.
    pub(crate) fn take_explanations(&self) -> Vec<RouteExplanation> {
        std::mem::take(&mut *self.explanations.borrow_mut())
            .into_values()
            .collect()
    }

    /// Record what the latest evaluation of `key`'s statement states of its
    /// type: the solver sweeps until nothing moves, so the settled sweep's
    /// evaluation — the one over the final inputs — is the one that stays.
    /// `None` removes what an earlier sweep stated.
    pub(crate) fn record_folded(&self, key: ValueKey, folded: Option<FoldedType>) {
        let mut map = self.folded.borrow_mut();
        match folded {
            Some(folded) => {
                map.insert(key, folded);
            }
            None => {
                map.remove(&key);
            }
        }
    }

    /// The folded type recorded for `key` so far.
    pub(crate) fn folded_of(&self, key: ValueKey) -> Option<FoldedType> {
        self.folded.borrow().get(&key).cloned()
    }

    /// Forget every folded type: a barrier widens every value the run
    /// holds, and a value it widened states nothing of its type.
    pub(crate) fn forget_folded(&self) {
        self.folded.borrow_mut().clear();
    }

    /// Every folded type the run holds at its end.
    pub(crate) fn take_folded_types(&self) -> HashMap<ValueKey, FoldedType> {
        std::mem::take(&mut *self.folded.borrow_mut())
    }

    /// Count one entry into the direct-evaluator family.
    fn enter_direct(&self) {
        let mut tally = self.tally.get();
        tally.direct += 1;
        self.tally.set(tally);
    }

    /// Count one entry into the expression-engine family.
    fn enter_expression(&self) {
        let mut tally = self.tally.get();
        tally.expression += 1;
        self.tally.set(tally);
    }

    /// Count one entry into a declared implementation.
    fn enter_implementation(&self) {
        let mut tally = self.tally.get();
        tally.implementation += 1;
        self.tally.set(tally);
    }

    /// The run's route-entry counts, direct, expression and implementation
    /// dispatches alike, nested entries included.
    pub(crate) fn take_route_tally(&self) -> RouteTally {
        self.tally.take()
    }

    /// Zero the tally, for the start of one full sweep over the CFG's
    /// blocks. The fixed point re-evaluates every executable statement
    /// each sweep until its answers stop changing, and once more to
    /// finalise, so counting every call would report a multiple of the
    /// true entry count; resetting at the top of each sweep keeps only the
    /// last one's, which is the settled answer's. The branch-fold pass that
    /// follows the fixed point adds to that settled count.
    pub(crate) fn reset_tally_for_sweep(&self) {
        self.tally.set(RouteTally::default());
    }

    /// Open the next solver pass: one tenth of what the request has left
    /// (`docs/design/compiler/value-evaluation.md` § *The three nested
    /// budgets*).
    pub(crate) fn open_iteration(&self) {
        let iteration = self.request.borrow_mut().iteration();
        *self.iteration.borrow_mut() = iteration;
    }

    /// The per-evaluation budget a route runs under, charging through the
    /// current pass and the run's request.
    fn budget(&self) -> Budget {
        self.iteration.borrow().evaluation_within()
    }

    /// A driver for a statement evaluated outside a run: the fold inputs'
    /// registry when there are any, else the process-wide default.
    pub(crate) fn detached(folds: Option<BuiltinFoldInputs<'a>>, policy: FoldPolicy) -> Self {
        let registry = match folds {
            Some(f) => f.registry,
            None => tcl_registry::default_registry(),
        };
        Self::new(
            TraceInputs {
                registry,
                traced_variables: &EMPTY_NAMES,
                has_dynamic_variable_trace: false,
                analysis_context: None,
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
    fn trusted(&self, head: &str) -> bool {
        self.folds.is_some_and(|f| match f.trust {
            FoldTrust::WholeModule => f.mutations.trusts(head),
            FoldTrust::ObservedBindings => f.mutations.observed_binding_is_the_builtin(head),
        })
    }

    /// Resolve `head args…` through the invocation resolver under the
    /// registry's own surface.
    fn resolve<'w>(
        &self,
        head: &'w str,
        words: &'w [InvocationWord<'w>],
    ) -> Option<ResolvedInvocation<'a, 'w>> {
        self.registry
            .resolve_structured_invocation(
                InvocationWords::structured(InvocationWord::Literal(head), words),
                self.registry.own_surface_query(),
            )
            .resolved()
    }

    /// The typed `Incr` statement: its node projects to the invocation view
    /// of the command whose lowering it is (`incr name ?amount?`), and the
    /// registry's derived cell update evaluates it. A braced amount is its
    /// own text (`incr x {$n}` raises in every release), never a read.
    /// Answers each of `defs`, the statement's definitions.
    pub(crate) fn evaluate_incr<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &self,
        name: &str,
        amount: Option<(&str, bool)>,
        defs: &[(String, ValueKey)],
        uses: &HashMap<Symbol, Version, S1>,
        values: &HashMap<ValueKey, LatticeValue, S2>,
        ssa: &SsaFunction,
    ) -> Vec<DefAnswer> {
        let heads = typed_node_commands(self.registry, LoweringHookId::Incr);
        let Some(&head) = heads.first() else {
            return widened(defs);
        };
        if !heads.iter().all(|head| self.trusted(head)) {
            self.explain(head, None, "declined: rebinding-suspected".to_owned());
            return widened(defs);
        }
        let texts: Vec<&str> = std::iter::once(name)
            .chain(amount.map(|(text, _)| text))
            .collect();
        let words: Vec<InvocationWord<'_>> = std::iter::once(InvocationWord::Literal(name))
            .chain(amount.map(amount_word))
            .collect();
        let Some(resolved) = self.resolve(head, &words) else {
            return widened(defs);
        };
        let Some(semantics) = resolved.semantics.value.semantics() else {
            return widened(defs);
        };
        let sources = std::iter::once(OperandSource::Literal)
            .chain(amount.map(|(text, braced)| {
                if braced {
                    OperandSource::BracedLiteral
                } else if matches!(word_of(text), InvocationWord::Dynamic) {
                    OperandSource::Substituted
                } else {
                    OperandSource::Literal
                }
            }))
            .collect();
        let view = view_of(&resolved, &texts, &words, InvocationLayout::Source);
        let inputs = LatticeInputs {
            driver: self,
            view,
            uses,
            values,
            ssa,
            sources,
        };
        self.call_defs(head, semantics, defs, &inputs)
    }

    /// The lattice values a resolved invocation leaves in `defs`, the
    /// variables its statement defines: the registry's evaluator over the
    /// lattice inputs, lifted over one finite input, on the route the
    /// registry owns; each outcome validated against the invocation's
    /// targets ([`validate_outcome`]) and applied per place
    /// ([`Self::apply_outcome`]), and the members joined per definition.
    fn call_defs(
        &self,
        head: &str,
        semantics: &dyn tcl_registry::value_transfer::CommandSemantics,
        defs: &[(String, ValueKey)],
        inputs: &dyn AnalysisInputs,
    ) -> Vec<DefAnswer> {
        let route = semantics.route();
        match route {
            EvalRoute::Direct { id } if id.owner() == EvaluatorOwner::Registry => {
                self.enter_direct();
            }
            // A declared implementation is registry-owned too: the
            // specialisation's `evaluate` runs it in the bounded host.
            EvalRoute::Implementation(_) => self.enter_implementation(),
            _ => {
                self.explain(
                    head,
                    Some(route),
                    "not evaluated: the route is not registry-owned".to_owned(),
                );
                return widened(defs);
            }
        }
        let answer = evaluate_lifted(semantics, inputs, &mut self.budget(), MAX_CONSTSET_SIZE);
        self.explain(head, Some(route), answer_label(&answer));
        let outcomes = match answer {
            LiftedAnswer::Pending => {
                return defs
                    .iter()
                    .map(|(_, key)| DefAnswer::untyped(*key, LatticeValue::Unknown))
                    .collect();
            }
            LiftedAnswer::Declined(_) => return widened(defs),
            LiftedAnswer::Evaluated(outcomes) => outcomes,
        };
        let plan = semantics.structure(inputs);
        let targets = semantics.store_targets(inputs);
        if let Some(reason) = outcomes
            .iter()
            .find_map(|outcome| validate_outcome(&plan, &targets, outcome).err())
        {
            self.explain(
                head,
                Some(route),
                format!("declined: {}", reason_label(reason)),
            );
            return widened(defs);
        }
        let mut joined: Option<Vec<DefAnswer>> = None;
        for outcome in &outcomes {
            let applied = match self.apply_outcome(outcome, defs, inputs) {
                Ok(applied) => applied,
                Err(reason) => {
                    self.explain(
                        head,
                        Some(route),
                        format!("declined: {}", reason_label(reason)),
                    );
                    return widened(defs);
                }
            };
            joined = Some(match joined {
                None => applied,
                Some(earlier) => earlier
                    .into_iter()
                    .zip(&applied)
                    .map(|(left, right)| left.join(right))
                    .collect(),
            });
        }
        joined.unwrap_or_else(|| widened(defs))
    }

    /// The value one outcome leaves in each of `defs`, the statement's
    /// definitions (`docs/design/compiler/value-transfers.md` § *Storage-
    /// writing commands*). Every store's target resolves to its place
    /// first, so two spellings of one cell are one place and a repeated
    /// target composes in execution order: a `Write` is its value, the
    /// last write winning; a `Preserve` keeps what the place holds at that
    /// point, the prior version's value when nothing earlier wrote it (a
    /// pending prior stays pending); a `MayWrite` and an `Unbind` widen,
    /// their facts being the type domain's and the existence rung's. A
    /// definition no store names widens.
    ///
    /// # Errors
    ///
    /// The whole answer declines, and every definition widens, for a
    /// completion other than the normal one (the prefix rule is slice
    /// 10's); a target that is no place (`DynamicName`, `EscapingPlace`
    /// from the resolver); an element write beside its array's base write
    /// (`OverlappingTargets`); and a traced place (`TracedPlace`), whose
    /// trace runs on the write and can observe or rewrite the others.
    fn apply_outcome(
        &self,
        outcome: &InvocationOutcome,
        defs: &[(String, ValueKey)],
        input: &dyn AnalysisInputs,
    ) -> Result<Vec<DefAnswer>, DeclineReason> {
        if outcome.completion != CompletionOutcome::Normal {
            return Err(DeclineReason::Unsupported);
        }
        let mut placed: Vec<(PlaceRef, &StoreOutcome)> =
            Vec::with_capacity(outcome.ordered_stores.len());
        for store in &outcome.ordered_stores {
            let place = input.place(store.target().0)?;
            if placed
                .iter()
                .any(|(seen, _)| seen.overlaps_as_element_and_base(&place))
            {
                return Err(DeclineReason::OverlappingTargets);
            }
            if self.is_traced(&place) {
                return Err(DeclineReason::TracedPlace);
            }
            placed.push((place, store));
        }
        Ok(defs
            .iter()
            .map(|(name, key)| {
                let named: Vec<&(PlaceRef, &StoreOutcome)> = placed
                    .iter()
                    .filter(|(place, _)| place.name == *name)
                    .collect();
                let Some((place, _)) = named.first() else {
                    return DefAnswer::untyped(*key, LatticeValue::Overdefined);
                };
                if self.is_escaping(place) {
                    return DefAnswer::untyped(*key, LatticeValue::Overdefined);
                }
                let mut held: Option<LatticeValue> = None;
                for (_, store) in &named {
                    match store {
                        StoreOutcome::Write { value, .. } => held = Some(exact_to_lattice(value)),
                        StoreOutcome::Preserve { .. } => {}
                        StoreOutcome::Unbind { .. } | StoreOutcome::MayWrite { .. } => {
                            held = Some(LatticeValue::Overdefined);
                        }
                    }
                }
                let value = held.unwrap_or_else(|| {
                    fact_to_lattice(&input.prior_store(place, FactDomain::ExactValue))
                });
                DefAnswer {
                    key: *key,
                    value,
                    folded: FoldedType::after_stores(
                        outcome,
                        named.iter().map(|(_, store)| *store),
                    ),
                }
            })
            .collect())
    }

    /// Whether a trace can observe `place`: the module names it in a trace,
    /// or traces a computed name.
    fn is_traced(&self, place: &PlaceRef) -> bool {
        self.context.has_dynamic_variable_trace
            || self.context.traced_variables.contains(&place.name)
            || self.context.traced_variables.contains(place.base())
    }

    /// Whether `place` is writable from outside the function — the
    /// solver's own rule ([`crate::sccp::is_externally_mutable`]), so a
    /// definition it widens before any transfer runs is widened here too.
    fn is_escaping(&self, place: &PlaceRef) -> bool {
        let escaping = &self.context.escaping;
        self.context.has_dynamic_variable_trace
            || place.name.starts_with("::")
            || escaping.contains(&place.name)
    }

    /// A `Call` statement's defs, each with its value. The synthetic loop
    /// header the CFG builder emits — the one `Call` carrying
    /// `foreach_groups` — has a declared structure the solver applies: its
    /// iteration plan's single list binder takes the set of the list's
    /// elements. Any other resolved call takes the ordered stores its
    /// registry-owned evaluation makes ([`Self::apply_outcome`]); every
    /// other call keeps the conservative answer.
    pub(crate) fn evaluate_call<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &self,
        stmt_ssa: &SsaStatement,
        values: &HashMap<ValueKey, LatticeValue, S2>,
        ssa: &SsaFunction,
        uses: &HashMap<Symbol, Version, S1>,
    ) -> Vec<DefAnswer> {
        let defs = named_defs(stmt_ssa, ssa);
        let Statement::Call {
            args,
            defs: binders,
            tokens,
            foreach_groups,
            ..
        } = &stmt_ssa.statement
        else {
            return widened(&defs);
        };
        let head = stmt_ssa.statement.canonical_command_or_source();
        if !self.trusted(head) {
            self.explain(head, None, "declined: rebinding-suspected".to_owned());
            return widened(&defs);
        }
        if foreach_groups.is_none() {
            let cooked = call_arguments(args, tokens.as_ref(), &self.lexer_config);
            return self.evaluate_source_call(head, &cooked, &defs, uses, values, ssa);
        }
        let value = self.evaluate_loop_header(head, args, binders, uses, values, ssa);
        defs.iter()
            .map(|(_, key)| DefAnswer::untyped(*key, value.clone()))
            .collect()
    }

    /// The synthetic loop header's value for its binders: the iteration
    /// plan's single list binder takes the set of the list's elements; a
    /// multi-variable or multi-list header stays `Overdefined`.
    fn evaluate_loop_header<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &self,
        head: &str,
        args: &[String],
        defs: &[String],
        uses: &HashMap<Symbol, Version, S1>,
        values: &HashMap<ValueKey, LatticeValue, S2>,
        ssa: &SsaFunction,
    ) -> LatticeValue {
        let texts: Vec<&str> = args.iter().map(String::as_str).collect();
        let words: Vec<InvocationWord<'_>> = texts.iter().copied().map(word_of).collect();
        let Some(resolved) = self.resolve(head, &words) else {
            return LatticeValue::Overdefined;
        };
        let Some(semantics) = resolved.semantics.value.semantics() else {
            self.explain(head, None, "declined: no-semantics".to_owned());
            return LatticeValue::Overdefined;
        };
        let view = view_of(
            &resolved,
            &texts,
            &words,
            InvocationLayout::LoopHeader { binders: defs },
        );
        let inputs = LatticeInputs {
            driver: self,
            view,
            uses,
            values,
            ssa,
            sources: words
                .iter()
                .map(|word| match word {
                    InvocationWord::Literal(_) => OperandSource::Literal,
                    _ => OperandSource::Unknown,
                })
                .collect(),
        };
        let PlanAnswer::Iterate(plan) = semantics.structure(&inputs) else {
            return LatticeValue::Overdefined;
        };
        // One binder over one list: the per-element transfer. A
        // multi-variable or multi-list header stays `Overdefined`.
        let (IterableKind::List(iterable), 1) = (&plan.iterable, plan.binders.len()) else {
            return LatticeValue::Overdefined;
        };
        let Some(list) = texts.get(iterable.0) else {
            return LatticeValue::Overdefined;
        };
        let rules = self.policy.word_rules;
        let elements = extract_foreach_elements(list, rules)
            .or_else(|| resolve_foreach_list_via_lattice(list, uses, values, ssa, rules))
            .or_else(|| {
                // `foreach v [list a b c]` — fold the command substitution
                // to a constant list string, then split into elements.
                let arg = list.trim();
                if arg.starts_with('[')
                    && arg.ends_with(']')
                    && let Some((LatticeValue::Const(ConstValue::String(s)), _)) =
                        self.fold_cmd_subst_routes(arg, uses, values, ssa)
                {
                    return Some(split_list_values(&s, rules));
                }
                None
            });
        match elements {
            Some(items) if items.is_empty() => LatticeValue::Overdefined,
            Some(items) => {
                let consts: Vec<ConstValue> = items
                    .iter()
                    .map(|s| exact_to_const(&ExactValue::from_literal(s)))
                    .collect();
                if consts.len() == 1 {
                    LatticeValue::Const(consts.into_iter().next().unwrap())
                } else {
                    LatticeValue::constset(consts)
                }
            }
            None => LatticeValue::Overdefined,
        }
    }

    /// A call in its source layout, its arguments read as source words: the
    /// stores its registry-owned evaluation makes, applied to the variables
    /// it defines ([`Self::call_defs`]).
    fn evaluate_source_call<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &self,
        head: &str,
        cooked: &[ArgWord<'_>],
        defs: &[(String, ValueKey)],
        uses: &HashMap<Symbol, Version, S1>,
        values: &HashMap<ValueKey, LatticeValue, S2>,
        ssa: &SsaFunction,
    ) -> Vec<DefAnswer> {
        let texts: Vec<&str> = cooked.iter().map(|arg| arg.text.as_ref()).collect();
        let words: Vec<InvocationWord<'_>> = cooked.iter().map(ArgWord::word).collect();
        let Some(resolved) = self.resolve(head, &words) else {
            return widened(defs);
        };
        let Some(semantics) = resolved.semantics.value.semantics() else {
            self.explain(head, None, "declined: no-semantics".to_owned());
            return widened(defs);
        };
        let view = view_of(&resolved, &texts, &words, InvocationLayout::Source);
        let inputs = LatticeInputs {
            driver: self,
            view,
            uses,
            values,
            ssa,
            sources: cooked.iter().map(|arg| arg.source).collect(),
        };
        self.call_defs(head, semantics, defs, &inputs)
    }

    /// Fold a `[cmd args…]` command substitution through the resolved
    /// command's declared route, then — when the caller holds the trust
    /// fact — through the registry's constant-fold engine. The value comes
    /// with the folded type the route's evaluation states; the engine's
    /// text states none.
    pub(crate) fn fold_cmd_subst<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &self,
        value: &str,
        uses: &HashMap<Symbol, Version, S1>,
        values: &HashMap<ValueKey, LatticeValue, S2>,
        ssa: &SsaFunction,
    ) -> Option<(LatticeValue, Option<FoldedType>)> {
        if let Some(folded) = self.fold_cmd_subst_routes(value, uses, values, ssa) {
            return Some(folded);
        }
        // Registry const-fold fallback: the fold's `$var` words resolve at
        // this statement's use versions, so a folded value re-enters the
        // lattice and downstream statements see it — the multi-hop chain
        // the declared routes cannot close yet. Checked after them so
        // single-hop results stay byte-identical, and only for a caller that
        // turns the engine on: the shared per-unit lattice holds the trust
        // fact with the engine off, so its fold surface stays the routes'.
        let f = self.folds.filter(|f| f.registry_engine)?;
        let inner = value.strip_prefix('[')?.strip_suffix(']')?;
        let trusts = |name: &str| f.mutations.trusts(name);
        let lookup = |name: &str| lattice_const_text(name, uses, values, ssa);
        let folded = crate::const_subst::ConstSubstCtx {
            registry: f.registry,
            resolution_namespace: "::",
            version: f
                .dialect
                .and_then(tcl_dialect::DialectProfile::const_fold_version),
            defining_class: f.defining_class,
            trusts: &trusts,
            lookup_var: &lookup,
        }
        .fold_cmd_subst(inner)?;
        Some((exact_to_lattice(&ExactValue::from_literal(&folded)), None))
    }

    /// The declared-route half of [`Self::fold_cmd_subst`]: the one command
    /// the substitution holds, run on its declared route
    /// ([`Self::run_script`]) under the effect-free nested policy, with the
    /// folded type every member's result states ([`FoldedType::join_all`]).
    fn fold_cmd_subst_routes<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &self,
        value: &str,
        uses: &HashMap<Symbol, Version, S1>,
        values: &HashMap<ValueKey, LatticeValue, S2>,
        ssa: &SsaFunction,
    ) -> Option<(LatticeValue, Option<FoldedType>)> {
        let inner = value.strip_prefix('[')?.strip_suffix(']')?;
        let run = self.run_script(inner, uses, values, ssa)?;
        // Binding validity comes first: after `rename list mylist` or a
        // shadowing `proc format …` anywhere in the unit, `[list a 1]` is a
        // call to something else entirely, and the const-fold engine the
        // caller falls back to asks the same question itself.
        if run.rebound {
            return None;
        }
        // In value position a nested invocation runs under the effect-free
        // policy: an outcome with stores has no definition to land on here.
        let folded = match &run.answer {
            LiftedAnswer::Evaluated(outcomes)
                if outcomes.iter().all(|outcome| !outcome.has_stores()) =>
            {
                Some(lattice_of_outcomes(outcomes, result_of))
                    .filter(|folded| *folded != LatticeValue::Overdefined)
                    .map(|folded| {
                        let typed = FoldedType::join_all(
                            outcomes
                                .iter()
                                .map(|outcome| FoldedType::of_result(outcome)),
                        );
                        (folded, typed)
                    })
            }
            LiftedAnswer::Pending => Some((LatticeValue::Unknown, None)),
            LiftedAnswer::Evaluated(_) | LiftedAnswer::Declined(_) => None,
        };
        self.explain(
            &run.head,
            run.route,
            match (&run.answer, &folded) {
                (LiftedAnswer::Evaluated(_), None) => {
                    "not substituted: the outcome writes storage".to_owned()
                }
                _ => answer_label(&run.answer),
            },
        );
        folded
    }

    /// The one command a `[…]` script holds, resolved and run on its
    /// declared route over this statement's lattice inputs: a
    /// registry-owned evaluator, the expression engine, or a transitional
    /// handler. `None` when the script is not one command the registry
    /// resolves to a declaration.
    fn run_script<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &self,
        script: &str,
        uses: &HashMap<Symbol, Version, S1>,
        values: &HashMap<ValueKey, LatticeValue, S2>,
        ssa: &SsaFunction,
    ) -> Option<ScriptRun> {
        let commands =
            crate::segmenter::segment_commands_with_offset_and_config(script, 0, self.lexer_config);
        let [seg] = commands.as_slice() else {
            return None;
        };
        let head = seg.name();
        if split_head(script).0 != head {
            return None;
        }
        if !self.trusted(head) {
            return Some(ScriptRun {
                head: head.to_owned(),
                route: None,
                answer: LiftedAnswer::Declined(DeclineReason::RebindingSuspected),
                binding: binding_of(head, head),
                rebound: true,
            });
        }
        let cooked: Vec<ArgWord<'_>> = seg
            .arg_tokens()
            .iter()
            .zip(seg.arg_single_token())
            .zip(seg.args())
            .map(|((token, &single), text)| {
                ArgWord::of_token(text, token.kind, single, &self.lexer_config)
            })
            .collect();
        let texts: Vec<&str> = cooked.iter().map(|arg| arg.text.as_ref()).collect();
        let words: Vec<InvocationWord<'_>> = cooked.iter().map(ArgWord::word).collect();
        let resolved = self.resolve(head, &words)?;
        let semantics = resolved.semantics.value.semantics()?;
        let binding = binding_of(head, resolved.canonical_command);
        let view = view_of(&resolved, &texts, &words, InvocationLayout::Source);
        let inputs = LatticeInputs {
            driver: self,
            view,
            uses,
            values,
            ssa,
            sources: cooked.iter().map(|arg| arg.source).collect(),
        };
        let route = semantics.route();
        let answer = match route {
            EvalRoute::Direct { id } => {
                self.enter_direct();
                match id.owner() {
                    EvaluatorOwner::Registry => {
                        evaluate_lifted(semantics, &inputs, &mut self.budget(), MAX_CONSTSET_SIZE)
                    }
                    // No compiler-owned evaluator remains: a route the
                    // registry does not own evaluates nothing here.
                    EvaluatorOwner::Transitional { .. } => {
                        LiftedAnswer::Declined(DeclineReason::Unsupported)
                    }
                }
            }
            EvalRoute::Expression { language } => {
                self.enter_expression();
                // A pack's option row can switch the declared route off;
                // the engine adapter never reads the declaration, so the
                // driver asks it first.
                match semantics
                    .as_declared()
                    .and_then(|declared| declared.option_decline(&inputs))
                {
                    Some(EvalAnswer::Pending) => LiftedAnswer::Pending,
                    Some(EvalAnswer::Declined(reason)) => LiftedAnswer::Declined(reason),
                    Some(EvalAnswer::Evaluated(_)) | None => {
                        let expression = ExpressionEvaluation {
                            expression: Expression::Assembled(ExpressionRoute { language }),
                            policy: self.policy,
                            head: Some(binding.clone()),
                        };
                        evaluate_lifted(&expression, &inputs, &mut self.budget(), MAX_CONSTSET_SIZE)
                    }
                }
            }
            EvalRoute::None { reason } => LiftedAnswer::Declined(DeclineReason::NoRoute(reason)),
            // The specialisation's `evaluate` resolves the declared inputs
            // and runs the body in this thread's host.
            EvalRoute::Implementation(_) => {
                self.enter_implementation();
                evaluate_lifted(semantics, &inputs, &mut self.budget(), MAX_CONSTSET_SIZE)
            }
        };
        Some(ScriptRun {
            head: head.to_owned(),
            route: Some(route),
            answer,
            binding,
            rebound: false,
        })
    }

    /// The nested-substitution service: `script`'s one command under
    /// `state`'s policy, its binding and every binding its answer rests on
    /// recorded in the state's evidence. Only an effect-free outcome is
    /// admitted — any store is `StatefulNested` — and an outcome that
    /// differs between the members of a finite input declines as
    /// correlated: the host evaluation holds one value per operand.
    fn nested_answer<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &self,
        script: &str,
        state: &mut EvaluationState,
        uses: &HashMap<Symbol, Version, S1>,
        values: &HashMap<ValueKey, LatticeValue, S2>,
        ssa: &SsaFunction,
    ) -> EvalAnswer {
        let depth = self.nesting.get();
        if depth >= Budget::EVALUATION_DEPTH {
            return EvalAnswer::Declined(DeclineReason::Budget(BudgetLimit::Depth));
        }
        self.nesting.set(depth + 1);
        let run = self.run_script(script, uses, values, ssa);
        self.nesting.set(depth);
        let Some(run) = run else {
            return EvalAnswer::Declined(DeclineReason::Unsupported);
        };
        record_binding(&mut state.evidence, run.binding);
        let outcomes = match run.answer {
            LiftedAnswer::Pending => return EvalAnswer::Pending,
            LiftedAnswer::Declined(reason) => return EvalAnswer::Declined(reason),
            LiftedAnswer::Evaluated(outcomes) => outcomes,
        };
        if outcomes.iter().any(|outcome| outcome.has_stores()) {
            return EvalAnswer::Declined(DeclineReason::StatefulNested);
        }
        let mut outcomes = outcomes.into_iter();
        let Some(first) = outcomes.next() else {
            return EvalAnswer::Declined(DeclineReason::Unsupported);
        };
        if outcomes.any(|outcome| outcome.result != first.result) {
            return EvalAnswer::Declined(DeclineReason::CorrelatedSets);
        }
        for binding in &first.evidence.bindings {
            record_binding(&mut state.evidence, binding.clone());
        }
        EvalAnswer::Evaluated(first)
    }

    /// A fused `set name [expr {…}]` (`AssignExpr`): the parsed expression
    /// evaluated by the shared engine over this statement's lattice inputs,
    /// lifted over one finite input. The nested `expr` is the head whose
    /// binding the answer rests on; without a trust fact the fused node's
    /// own lowering stands.
    pub(crate) fn evaluate_assign_expr<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &self,
        expr: &ExprNode,
        command_binding: Option<&tcl_runtime_api::CommandBindingIdentity>,
        uses: &HashMap<Symbol, Version, S1>,
        values: &HashMap<ValueKey, LatticeValue, S2>,
        ssa: &SsaFunction,
    ) -> LatticeValue {
        let head = command_binding.map_or("expr", |binding| binding.name.as_str());
        if self.folds.is_some() && !self.trusted(head) {
            self.explain(head, None, "declined: rebinding-suspected".to_owned());
            return LatticeValue::Overdefined;
        }
        self.enter_expression();
        let expression = ExpressionEvaluation {
            expression: Expression::Parsed(expr),
            policy: self.policy,
            head: Some(binding_of(
                head,
                command_binding.map_or("expr", |binding| binding.identity.as_str()),
            )),
        };
        let answer = self.evaluate_expression_at(&expression, uses, values, ssa);
        self.explain(head, Some(expression.route()), answer_label(&answer));
        match answer {
            LiftedAnswer::Pending => LatticeValue::Unknown,
            LiftedAnswer::Declined(_) => LatticeValue::Overdefined,
            LiftedAnswer::Evaluated(outcomes) => lattice_of_outcomes(&outcomes, result_of),
        }
    }

    /// A branch condition's truth over the lattice inputs `uses` selects:
    /// the shared engine's full value read as Tcl reads a condition, per
    /// member of one finite input. `Some` only when every member agrees; an
    /// undecided, mixed, pending or declined answer is `None`, and the
    /// answer is recorded against the condition.
    pub(crate) fn evaluate_condition<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &self,
        condition: &ExprNode,
        uses: &HashMap<Symbol, Version, S1>,
        values: &HashMap<ValueKey, LatticeValue, S2>,
        ssa: &SsaFunction,
    ) -> Option<bool> {
        self.enter_expression();
        let expression = ExpressionEvaluation {
            expression: Expression::Parsed(condition),
            policy: self.policy,
            head: None,
        };
        let answer = self.evaluate_expression_at(&expression, uses, values, ssa);
        self.explain("condition", Some(expression.route()), answer_label(&answer));
        let LiftedAnswer::Evaluated(outcomes) = answer else {
            return None;
        };
        let mut decided = None;
        for outcome in &outcomes {
            let truth = truth_of(&outcome.result)?;
            match decided {
                None => decided = Some(truth),
                Some(earlier) if earlier != truth => return None,
                Some(_) => {}
            }
        }
        decided
    }

    /// The math-function service: the binding an `expr` call `name(…)`
    /// dispatches to. The function must exist in the target's `expr`
    /// grammar (`min` is 8.5's, the `is…` classes 9.0's); a name the
    /// grammar does not have is the program's error. Where the functions are
    /// commands (`::tcl::mathfunc::NAME`, from 8.5) the module can rebind
    /// one, and a rebound or shadowed wrapper is `RebindingSuspected` under
    /// the run's trust stance; under 8.4 the grammar dispatches internally,
    /// so no `proc` reaches it. Without a trust fact the builtin table
    /// stands, as it does for the fused node's own lowering.
    fn math_function(&self, name: &str) -> Result<BindingIdentity, DeclineReason> {
        let profile = self.context.profile;
        // A profile naming no release calls only what every release has.
        let available = tcl_syntax::expr::mathfunc::added_in(name).is_some_and(|since| {
            profile.is_none_or(|profile| since <= crate::tcl_expr_eval::fold_math_ceiling(profile))
        });
        if !available {
            return Err(DeclineReason::Unsupported);
        }
        let qualified = tcl_registry::mathfunc::qualified_name(name);
        let wrappers = profile.is_none_or(tcl_registry::mathfunc::command_wrappers_available);
        if wrappers && self.folds.is_some() && !self.trusted(&qualified) {
            return Err(DeclineReason::RebindingSuspected);
        }
        Ok(binding_of(name, &qualified))
    }

    /// `expression` evaluated over the lattice inputs `uses` selects, with
    /// no operand words of its own: the parsed expression reads every
    /// variable by name.
    fn evaluate_expression_at<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &self,
        expression: &ExpressionEvaluation<'_>,
        uses: &HashMap<Symbol, Version, S1>,
        values: &HashMap<ValueKey, LatticeValue, S2>,
        ssa: &SsaFunction,
    ) -> LiftedAnswer {
        let inputs = LatticeInputs {
            driver: self,
            view: ResolvedInvocationView {
                canonical_command: "expr",
                subcommand: None,
                form: None,
                layout: InvocationLayout::Source,
                operands: Vec::new(),
                argument_offset: 0,
            },
            uses,
            values,
            ssa,
            sources: Vec::new(),
        };
        evaluate_lifted(expression, &inputs, &mut self.budget(), MAX_CONSTSET_SIZE)
    }
}

static EMPTY_NAMES: BTreeSet<String> = BTreeSet::new();

/// The inputs of an expression a rewrite evaluates outside a solver run:
/// its `$name` reads answer from `constants`, a nested command declines, and
/// a math function asks the driver's binding service under the rewrite's
/// trust stance.
struct DetachedExpressionInputs<'a, 'd> {
    driver: &'a LatticeDriver<'d>,
    view: ResolvedInvocationView<'a>,
    constants: &'a HashMap<String, ExactValue>,
}

impl AnalysisInputs for DetachedExpressionInputs<'_, '_> {
    fn invocation(&self) -> &ResolvedInvocationView<'_> {
        &self.view
    }

    fn operand(&self, _id: OperandId, _domain: FactDomain) -> FactView {
        FactView::Top(DeclineReason::NotExact)
    }

    fn place(&self, _id: OperandId) -> Result<PlaceRef, DeclineReason> {
        Err(DeclineReason::Unsupported)
    }

    fn variable(&self, name: &str, domain: FactDomain) -> FactView {
        if domain != FactDomain::ExactValue {
            return FactView::Top(DeclineReason::Unavailable(AnalysisTier::Fast));
        }
        self.constants
            .get(name)
            .map_or(FactView::Top(DeclineReason::NotExact), |value| {
                FactView::Exact(value.clone(), None)
            })
    }

    fn prior_store(&self, place: &PlaceRef, domain: FactDomain) -> FactView {
        self.variable(&place.name, domain)
    }

    fn word_structure(&self, _id: OperandId) -> Result<WordStructure, DeclineReason> {
        Err(DeclineReason::Unsupported)
    }

    fn body(&self, _id: OperandId) -> Result<BodyRegion, DeclineReason> {
        Err(DeclineReason::Unsupported)
    }

    fn nested(&self, _script: &str, _state: &mut EvaluationState) -> EvalAnswer {
        EvalAnswer::Declined(DeclineReason::Unsupported)
    }

    fn math_function(&self, name: &str) -> Result<BindingIdentity, DeclineReason> {
        self.driver.math_function(name)
    }

    fn context(&self) -> &AnalysisContext {
        &self.driver.context
    }
}

/// `node` evaluated by the shared expression route outside a solver run —
/// the value the lattice proves for it over `constants`, or `None` wherever
/// the route declines. It is the one evaluator a rewrite asks
/// (`docs/design/compiler/value-transfers.md` § *`expr`: the first demanding
/// client*): a math function the target lacks, the module rebinds, or Tcl
/// does not call (`ABS`); a beyond-wide integer or an infinity the target
/// does not widen to; a numeral the target's grammar cannot decide — each
/// declines as the lattice does. `folds` carries the rewrite's whole-module
/// trust, and a nested command is never evaluated.
pub(crate) fn evaluate_expression_detached(
    node: &ExprNode,
    constants: &HashMap<String, ExactValue>,
    folds: BuiltinFoldInputs<'_>,
    policy: FoldPolicy,
) -> Option<ExactValue> {
    let driver = LatticeDriver::detached(Some(folds), policy);
    let expression = ExpressionEvaluation {
        expression: Expression::Parsed(node),
        policy,
        head: None,
    };
    let inputs = DetachedExpressionInputs {
        driver: &driver,
        view: ResolvedInvocationView {
            canonical_command: "expr",
            subcommand: None,
            form: None,
            layout: InvocationLayout::Source,
            operands: Vec::new(),
            argument_offset: 0,
        },
        constants,
    };
    match evaluate_lifted(
        &expression,
        &inputs,
        &mut driver.budget(),
        MAX_CONSTSET_SIZE,
    ) {
        LiftedAnswer::Evaluated(outcomes) => match outcomes.as_slice() {
            [outcome] => match &outcome.result {
                ExactValueOrUnavailable::Exact(value) => Some(value.clone()),
                ExactValueOrUnavailable::Unavailable(_) => None,
            },
            _ => None,
        },
        LiftedAnswer::Pending | LiftedAnswer::Declined(_) => None,
    }
}

/// Whether the condition `node` is decided outside a solver run: its value
/// on the shared expression route ([`evaluate_expression_detached`]) read
/// as `if` reads a truth, or `None` wherever the route declines or the value
/// is no truth.
pub(crate) fn decide_condition_detached(
    node: &ExprNode,
    constants: &HashMap<String, ExactValue>,
    folds: BuiltinFoldInputs<'_>,
    policy: FoldPolicy,
) -> Option<bool> {
    let value = evaluate_expression_detached(node, constants, folds, policy)?;
    truth_of(&ExactValueOrUnavailable::Exact(value))
}

/// One `[…]` script run on its declared route.
struct ScriptRun {
    /// The command's head as the script spells it.
    head: String,
    /// The declared route, when the head is still the builtin.
    route: Option<EvalRoute>,
    /// The route's answer, lifted over one finite input.
    answer: LiftedAnswer,
    /// The binding the answer rests on.
    binding: BindingIdentity,
    /// Whether the head no longer denotes its registry command.
    rebound: bool,
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

/// A condition's truth as `if` reads it: a number is true when non-zero, a
/// boolean word is its value, and anything else raises (`expected boolean
/// value`), which decides nothing.
fn truth_of(result: &ExactValueOrUnavailable) -> Option<bool> {
    let ExactValueOrUnavailable::Exact(value) = result else {
        return None;
    };
    match value.numeric {
        Some(NumericValue::Int(i)) => Some(i != 0),
        Some(NumericValue::Float(f)) if f.is_nan() => None,
        Some(NumericValue::Float(f)) => Some(f != 0.0),
        Some(NumericValue::Bool(b)) => Some(b),
        None => {
            let text = value.as_str().ok()?;
            if let Some(word) = tcl_syntax::boolean::parse_boolean_word(text) {
                return Some(word);
            }
            // A beyond-wide integer's canonical spelling: non-zero is true.
            // Only the canonical spelling: a leading zero is a text the
            // target's grammar did not read as a number (`08` under 8.x),
            // and `if` raises `expected boolean value` on it.
            let digits = text.strip_prefix('-').unwrap_or(text);
            (!digits.is_empty()
                && digits.bytes().all(|b| b.is_ascii_digit())
                && (digits == "0" || !digits.starts_with('0')))
            .then(|| digits != "0")
        }
    }
}

/// A pure outcome: `value` as the result, no store, and the evidence of the
/// route that computed it.
fn pure_outcome(
    value: ExactValue,
    route: EvalRoute,
    implementation: &'static str,
    revision: u64,
    bindings: Vec<BindingIdentity>,
) -> InvocationOutcome {
    InvocationOutcome {
        completion: CompletionOutcome::Normal,
        result: ExactValueOrUnavailable::Exact(value),
        ordered_stores: Vec::new(),
        types: TypeFacts::default(),
        evidence: DependencyEvidence {
            bindings,
            route: Some(RouteIdentity {
                route,
                implementation,
                revision,
            }),
            ..DependencyEvidence::default()
        },
    }
}

/// Which expression an [`ExpressionEvaluation`] evaluates.
enum Expression<'e> {
    /// The expression the route assembles from the invocation's argument
    /// words ([`ExpressionRoute::assemble`]).
    Assembled(ExpressionRoute),
    /// An expression the lowering already parsed: a fused assignment's, or
    /// a branch condition.
    Parsed(&'e ExprNode),
}

/// The expression route as the lift runs it: the shared engine over the
/// analysis services ([`evaluate_expression`]), so a finite input the
/// expression reads is pinned per member like any route's operand.
struct ExpressionEvaluation<'e> {
    /// The expression.
    expression: Expression<'e>,
    /// The value semantics the engine evaluates under.
    policy: FoldPolicy,
    /// The `expr` binding the answer rests on; a branch condition is read
    /// by its command's own expression parser and rests on none.
    head: Option<BindingIdentity>,
}

impl ExpressionEvaluation<'_> {
    /// The language the engine evaluates.
    const fn language(&self) -> LanguageProfileId {
        match &self.expression {
            Expression::Assembled(route) => route.language,
            Expression::Parsed(_) => LanguageProfileId::TclExpr,
        }
    }
}

impl CommandSemantics for ExpressionEvaluation<'_> {
    fn identity(&self) -> &'static str {
        ExpressionRoute {
            language: self.language(),
        }
        .identity()
    }

    fn route(&self) -> EvalRoute {
        EvalRoute::Expression {
            language: self.language(),
        }
    }

    fn variable_reads(&self, input: &dyn AnalysisInputs) -> Vec<String> {
        match &self.expression {
            Expression::Assembled(route) => route.variable_reads(input),
            Expression::Parsed(node) => {
                let config = LexerConfig::for_profile(input.context().profile);
                let mut reads: Vec<String> = node
                    .vars_element_qualified_with_config(config)
                    .into_iter()
                    .collect();
                reads.sort_unstable();
                reads
            }
        }
    }

    fn evaluate(&self, input: &dyn AnalysisInputs, budget: &mut Budget) -> EvalAnswer {
        let assembled;
        let node = match &self.expression {
            Expression::Assembled(route) => {
                let source = match route.assemble(input) {
                    Ok(source) => source,
                    Err(answer) => return answer,
                };
                let text = match source.text() {
                    Ok(text) => text,
                    Err(reason) => return EvalAnswer::Declined(reason),
                };
                assembled = crate::expr_parser::parse_expr_for_profile(text, self.policy.dialect);
                &assembled
            }
            Expression::Parsed(node) => *node,
        };
        let mut state = EvaluationState::new(NestedPolicy::EffectFreeOnly);
        if let Some(head) = &self.head {
            record_binding(&mut state.evidence, head.clone());
        }
        let answer = {
            let mut services = ExprServices::new(input, &mut state, budget);
            evaluate_expression(node, &mut services, self.policy)
        };
        match answer {
            ExprAnswer::Pending => EvalAnswer::Pending,
            ExprAnswer::Declined(reason) => EvalAnswer::Declined(reason),
            ExprAnswer::Value(value) => {
                let mut outcome = pure_outcome(
                    value,
                    self.route(),
                    self.identity(),
                    ExpressionRoute::REVISION,
                    state.evidence.bindings,
                );
                outcome.evidence.numerals = self.policy.numbers;
                outcome.evidence.release =
                    tcl_registry::value_transfer::TargetSemantics::of(self.policy.dialect).release;
                EvalAnswer::Evaluated(Box::new(outcome))
            }
        }
    }
}

/// The commands a typed statement stands for: every registry command whose
/// lowering `hook` produces it, shortest spelling first so the head a
/// consumer resolves through is stable. A typed node records no spelling,
/// so these are what binding validity is asked about.
fn typed_node_commands(registry: &CommandRegistry, hook: LoweringHookId) -> Vec<&str> {
    let mut names: Vec<&str> = registry
        .command_names_for_semantic_operation(SemanticOperationId::StructuredLowering(hook))
        .collect();
    names.sort_unstable_by(|a, b| a.len().cmp(&b.len()).then_with(|| a.cmp(b)));
    names
}

/// The source-word classification of a typed `Incr`'s amount: a braced
/// amount is a literal whatever it spells, any other is classified by its
/// text ([`word_of`]).
fn amount_word((text, braced): (&str, bool)) -> InvocationWord<'_> {
    if braced {
        InvocationWord::Literal(text)
    } else {
        word_of(text)
    }
}

/// The source-word classification of one operand text: a literal unless
/// it substitutes.
fn word_of(text: &str) -> InvocationWord<'_> {
    if text.contains('$') || text.contains('[') {
        InvocationWord::Dynamic
    } else {
        InvocationWord::Literal(text)
    }
}

/// A single-token literal word's value as Tcl substitutes it: a bare or
/// quoted word (`Esc`) under the document's escape grammar, a braced word
/// (`Str`) with its backslash-newlines collapsed. The const-fold engine
/// (`const_subst.rs`) cooks a literal word by this same rule, so a
/// declared route and the engine read one value for one word; `None` for
/// any other token kind. A word's raw spelling is not its value: `"a\tb"`
/// is three characters and `{$x}` is not a read of `x`.
pub(crate) fn literal_token_value<'t>(
    text: &'t str,
    kind: TokenType,
    config: &LexerConfig,
) -> Option<Cow<'t, str>> {
    match kind {
        TokenType::Esc => Some(tcl_lexer::backslash_subst_in(text, config.escapes)),
        TokenType::Str => Some(WordValueRules::from_config(config).collapse_braced_word(text)),
        _ => None,
    }
}

/// What the source says about how an operand's word substitutes, for the
/// inputs that answer its structure ([`AnalysisInputs::word_structure`])
/// and a substituted word's value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OperandSource {
    /// A single braced word: its content is its value.
    BracedLiteral,
    /// A bare or quoted word with nothing to substitute: its cooked text is
    /// its value.
    Literal,
    /// A bare or quoted word with substitutions, spelled as the source
    /// wrote its content: the value is the parts' values concatenated.
    Substituted,
    /// Nothing is known of the word's quoting: an expansion, a braced
    /// compound, a word respelled after lowering, or no source at all.
    Unknown,
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

impl<'t> ArgWord<'t> {
    /// The word one segmented token stands for: a single-token literal is
    /// its cooked value, a `{*}` word expands, and any other word is
    /// dynamic — substituted from its spelling unless it is a braced
    /// compound, whose spelling is not its source.
    fn of_token(text: &'t str, kind: TokenType, single: bool, config: &LexerConfig) -> Self {
        if kind == TokenType::Expand {
            return Self::spelled(text, InvocationWordKind::Expanded, OperandSource::Unknown);
        }
        match literal_token_value(text, kind, config).filter(|_| single) {
            Some(value) => Self {
                text: value,
                kind: InvocationWordKind::Literal,
                source: if kind == TokenType::Str {
                    OperandSource::BracedLiteral
                } else {
                    OperandSource::Literal
                },
            },
            None if kind == TokenType::Str => {
                Self::spelled(text, InvocationWordKind::Dynamic, OperandSource::Unknown)
            }
            None => Self::spelled(
                text,
                InvocationWordKind::Dynamic,
                OperandSource::Substituted,
            ),
        }
    }

    const fn spelled(text: &'t str, kind: InvocationWordKind, source: OperandSource) -> Self {
        Self {
            text: Cow::Borrowed(text),
            kind,
            source,
        }
    }

    /// The resolver's view of the word.
    fn word(&self) -> InvocationWord<'_> {
        match self.kind {
            InvocationWordKind::Literal => InvocationWord::Literal(&self.text),
            InvocationWordKind::Dynamic => InvocationWord::Dynamic,
            InvocationWordKind::Expanded => InvocationWord::Expanded,
            InvocationWordKind::Opaque => InvocationWord::Opaque,
        }
    }
}

/// A call statement's arguments as source words. The call's token snapshot
/// says which argument was a braced, bare, or quoted literal; an argument
/// whose spelling differs from its source word (rewritten after lowering)
/// is dynamic. With no source words to consult, a spelling that
/// substitutes is dynamic and one holding a backslash is too: its quoting,
/// and so its value, is unknown.
fn call_arguments<'t>(
    args: &'t [String],
    tokens: Option<&CommandTokens>,
    config: &LexerConfig,
) -> Vec<ArgWord<'t>> {
    let source = tokens.filter(|tokens| {
        tokens.synthetic.is_none()
            && tokens.argv_kinds.len() == args.len() + 1
            && tokens.single_token_word.len() == args.len() + 1
            && tokens.argv_texts.len() == args.len() + 1
    });
    args.iter()
        .enumerate()
        .map(|(index, text)| {
            let Some(tokens) = source else {
                return match word_of(text) {
                    InvocationWord::Literal(_) if !text.contains('\\') => {
                        ArgWord::spelled(text, InvocationWordKind::Literal, OperandSource::Literal)
                    }
                    _ => {
                        ArgWord::spelled(text, InvocationWordKind::Dynamic, OperandSource::Unknown)
                    }
                };
            };
            let at = index + 1;
            let expanded = tokens
                .expand_word
                .as_ref()
                .and_then(|flags| flags.get(at))
                .copied()
                .unwrap_or(false);
            if expanded {
                ArgWord::spelled(text, InvocationWordKind::Expanded, OperandSource::Unknown)
            } else if tokens.argv_texts[at] == *text {
                ArgWord::of_token(
                    text,
                    tokens.argv_kinds[at],
                    tokens.single_token_word[at],
                    config,
                )
            } else {
                ArgWord::spelled(text, InvocationWordKind::Dynamic, OperandSource::Unknown)
            }
        })
        .collect()
}

/// The resolver's projection of `resolved` over `texts`: the canonical
/// names, the layout, and the operands with their kinds and roles. Roles
/// come from the effective descriptor — the resolver over literal words,
/// else the static roles — as the resolver's owned facts compute them.
fn view_of<'a>(
    resolved: &ResolvedInvocation<'_, '_>,
    texts: &[&'a str],
    words: &[InvocationWord<'a>],
    layout: InvocationLayout<'a>,
) -> ResolvedInvocationView<'a> {
    let semantics = &resolved.semantics;
    let offset = semantics.argument_offset;
    let literals: Option<Vec<&str>> = words.iter().map(|w| w.literal()).collect();
    let roles: Vec<(u8, ArgRole)> = match (semantics.arg_role_resolver, literals) {
        (Some(role_resolver), Some(literals)) => literals
            .get(offset..)
            .map_or_else(|| semantics.arg_roles.to_vec(), role_resolver),
        _ => semantics.arg_roles.to_vec(),
    };
    let role_at = |index: usize| {
        index
            .checked_sub(offset)
            .and_then(|i| u8::try_from(i).ok())
            .and_then(|i| roles.iter().find(|(at, _)| *at == i).map(|(_, role)| *role))
    };
    let operands = texts
        .iter()
        .zip(words)
        .enumerate()
        .map(|(index, (text, word))| OperandView {
            text,
            kind: word.kind(),
            role: role_at(index),
        })
        .collect();
    ResolvedInvocationView {
        canonical_command: resolved.canonical_command,
        subcommand: match resolved.subcommand {
            tcl_registry::SubcommandResolution::Exact(sub)
            | tcl_registry::SubcommandResolution::UniquePrefix(sub) => Some(sub.canonical_name),
            _ => None,
        },
        form: resolved.form.as_ref().map(|form| form.name),
        layout,
        operands,
        argument_offset: offset,
    }
}

/// The lattice's answer for one SSA value as a fact view.
fn lattice_to_fact(value: &LatticeValue, identity: Option<ValueIdentity>) -> FactView {
    match value {
        LatticeValue::Unknown => FactView::Pending,
        LatticeValue::Const(c) => FactView::Exact(const_to_exact(c), identity),
        LatticeValue::ConstSet(vs) => {
            FactView::Finite(vs.iter().map(const_to_exact).collect(), identity)
        }
        LatticeValue::Overdefined => FactView::Top(DeclineReason::NotExact),
    }
}

/// A lattice constant as an exact value: the text the constant renders
/// as, with its classification as the additional fact.
pub(crate) fn const_to_exact(c: &ConstValue) -> ExactValue {
    match c {
        ConstValue::Int(i) => ExactValue::int(*i),
        ConstValue::Float(f) => ExactValue {
            bytes: f.to_string().into_bytes(),
            numeric: Some(NumericValue::Float(*f)),
            representation: tcl_registry::value_transfer::RepresentationEvidence::Unknown,
        },
        ConstValue::Bool(b) => ExactValue {
            bytes: (if *b { "1" } else { "0" }).as_bytes().to_vec(),
            numeric: Some(NumericValue::Bool(*b)),
            representation: tcl_registry::value_transfer::RepresentationEvidence::Unknown,
        },
        ConstValue::String(s) => ExactValue::text(s.clone()),
    }
}

/// An exact value as a lattice constant: the classification when it has
/// one, else the exact text.
pub(crate) fn exact_to_const(value: &ExactValue) -> ConstValue {
    match value.numeric {
        Some(NumericValue::Int(i)) => ConstValue::Int(i),
        Some(NumericValue::Float(f)) => ConstValue::Float(f),
        Some(NumericValue::Bool(b)) => ConstValue::Bool(b),
        None => ConstValue::String(String::from_utf8_lossy(&value.bytes).into_owned()),
    }
}

/// An exact value's lattice projection; bytes that are not text decline.
fn exact_to_lattice(value: &ExactValue) -> LatticeValue {
    if value.numeric.is_none() && std::str::from_utf8(&value.bytes).is_err() {
        return LatticeValue::Overdefined;
    }
    LatticeValue::Const(exact_to_const(value))
}

/// The SSA identity of a value key, packed.
fn identity_of(key: ValueKey) -> ValueIdentity {
    ValueIdentity((u64::from(key.0.0) << 32) | u64::from(key.1))
}

/// The analyser's read-only inputs over the SCCP lattice at one statement.
struct LatticeInputs<'a, S1, S2> {
    driver: &'a LatticeDriver<'a>,
    view: ResolvedInvocationView<'a>,
    uses: &'a HashMap<Symbol, Version, S1>,
    values: &'a HashMap<ValueKey, LatticeValue, S2>,
    ssa: &'a SsaFunction,
    /// How each operand's word substitutes, in operand order.
    sources: Vec<OperandSource>,
}

impl<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher> LatticeInputs<'_, S1, S2> {
    /// The lattice fact for `name` at this statement's use version.
    fn named_fact(&self, name: &str) -> FactView {
        let Some(sym) = self.ssa.var_symbol(name) else {
            return FactView::Top(DeclineReason::NotExact);
        };
        let Some(&ver) = self.uses.get(&sym) else {
            return FactView::Top(DeclineReason::NotExact);
        };
        self.values
            .get(&(sym, ver))
            .map_or(FactView::Pending, |value| {
                lattice_to_fact(value, Some(identity_of((sym, ver))))
            })
    }
}

impl<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher> AnalysisInputs
    for LatticeInputs<'_, S1, S2>
{
    fn invocation(&self) -> &ResolvedInvocationView<'_> {
        &self.view
    }

    fn operand(&self, id: OperandId, domain: FactDomain) -> FactView {
        if domain != FactDomain::ExactValue {
            return FactView::Top(DeclineReason::Unavailable(self.driver.context.tier));
        }
        let Some(operand) = self.view.operand(id) else {
            return FactView::Top(DeclineReason::NotExact);
        };
        match operand.kind {
            // A literal word's text is its value, already cooked: never
            // re-read as a substitution (`{$x}` and `"\$x"` are the text
            // `$x`, not a read of `x`).
            InvocationWordKind::Literal => {
                FactView::Exact(ExactValue::from_literal(operand.text), None)
            }
            InvocationWordKind::Dynamic => match self.sources.get(id.0) {
                Some(OperandSource::Substituted) => self.substituted(operand.text),
                _ => simple_var_ref_name(operand.text)
                    .map_or(FactView::Top(DeclineReason::NotExact), |name| {
                        self.named_fact(name)
                    }),
            },
            InvocationWordKind::Expanded | InvocationWordKind::Opaque => {
                FactView::Top(DeclineReason::NotExact)
            }
        }
    }

    fn place(&self, id: OperandId) -> Result<PlaceRef, DeclineReason> {
        let text = self
            .view
            .operand(id)
            .map(|operand| operand.text)
            .ok_or(DeclineReason::NotExact)?;
        // A dynamic-key target (`incr a($i)`) never interns a symbol — that
        // miss is permanent, so it must widen: pending would launder a
        // fanned element's stale constant through the join.
        let name = crate::naming::element_var_name(text);
        if self.ssa.var_symbol(name).is_none() {
            return Err(DeclineReason::DynamicName);
        }
        Ok(place_named(name))
    }

    fn variable(&self, name: &str, domain: FactDomain) -> FactView {
        if domain != FactDomain::ExactValue {
            return FactView::Top(DeclineReason::Unavailable(self.driver.context.tier));
        }
        self.named_fact(name)
    }

    fn prior_store(&self, place: &PlaceRef, domain: FactDomain) -> FactView {
        if domain != FactDomain::ExactValue {
            return FactView::Top(DeclineReason::Unavailable(self.driver.context.tier));
        }
        let Some(sym) = self.ssa.var_symbol(&place.name) else {
            return FactView::Top(DeclineReason::DynamicName);
        };
        // A use the statement does not hold is a permanent miss, as a
        // dynamic key's is in `place`: a value-position cell update's read
        // sits on the synthetic call ahead of its host, so the host's uses
        // never reach it. Reading version 0 instead found no value and
        // answered a `Pending` that never resolved, which a phi then
        // laundered into the other arm's constant.
        let Some(&ver) = self.uses.get(&sym) else {
            return FactView::Top(DeclineReason::NotExact);
        };
        self.values
            .get(&(sym, ver))
            .map_or(FactView::Pending, |value| {
                lattice_to_fact(value, Some(identity_of((sym, ver))))
            })
    }

    fn word_structure(&self, id: OperandId) -> Result<WordStructure, DeclineReason> {
        let operand = self.view.operand(id).ok_or(DeclineReason::NotExact)?;
        let whole = |start: u32| {
            let end = u32::try_from(operand.text.len())
                .unwrap_or(u32::MAX)
                .saturating_add(start);
            vec![WordPart::Literal {
                span: Span::new(start, end),
                text: operand.text.to_owned(),
            }]
        };
        match self.sources.get(id.0) {
            // The content starts one past the opening brace.
            Some(OperandSource::BracedLiteral) => Ok(WordStructure {
                braced: true,
                parts: whole(1),
            }),
            Some(OperandSource::Literal) => Ok(WordStructure {
                braced: false,
                parts: whole(0),
            }),
            Some(OperandSource::Substituted) => Ok(WordStructure {
                braced: false,
                parts: word_parts(operand.text, self.driver.lexer_config)?,
            }),
            Some(OperandSource::Unknown) | None => Err(DeclineReason::NotExact),
        }
    }

    fn body(&self, _id: OperandId) -> Result<BodyRegion, DeclineReason> {
        Err(DeclineReason::Unsupported)
    }

    fn nested(&self, script: &str, state: &mut EvaluationState) -> EvalAnswer {
        self.driver
            .nested_answer(script, state, self.uses, self.values, self.ssa)
    }

    fn math_function(&self, name: &str) -> Result<BindingIdentity, DeclineReason> {
        self.driver.math_function(name)
    }

    fn context(&self) -> &AnalysisContext {
        &self.driver.context
    }
}

impl<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher> LatticeInputs<'_, S1, S2> {
    /// A substituted word's value: its parts' values concatenated — the
    /// literal runs decoded under the document's grammar, each variable
    /// read at this statement's use version, each script through the
    /// nested service under the effect-free policy. A lone variable read
    /// keeps the variable's own fact, identity included, so the lift can
    /// pin it. A part that is never exact makes the word never exact; else
    /// a pending part makes it pending.
    fn substituted(&self, text: &str) -> FactView {
        use tcl_lexer::word_parts::{SubstFlags, WordBody, WordPart as Part, decompose};
        if let Some(name) = simple_var_ref_name(text) {
            return self.named_fact(name);
        }
        let parts = match decompose(
            text.as_bytes(),
            SubstFlags::default(),
            self.driver.lexer_config,
        ) {
            WordBody::Literal(bytes) => {
                return FactView::Exact(exact_of_bytes(bytes.to_vec()), None);
            }
            WordBody::Parts(parts) => parts,
        };
        let mut bytes = Vec::with_capacity(text.len());
        let mut pending = false;
        for part in parts {
            let value = match part {
                Part::Text(run) => {
                    bytes.extend_from_slice(&run);
                    continue;
                }
                Part::Variable(reference) => match variable_name(&reference) {
                    Ok(name) => self.named_fact(&name).exact(),
                    Err(reason) => return FactView::Top(reason),
                },
                Part::Command(script) => match std::str::from_utf8(script) {
                    Ok(script) => {
                        let mut state = EvaluationState::new(NestedPolicy::EffectFreeOnly);
                        match self.nested(script, &mut state) {
                            EvalAnswer::Evaluated(outcome) => match outcome.result {
                                ExactValueOrUnavailable::Exact(value) => Ok(value),
                                ExactValueOrUnavailable::Unavailable(_) => {
                                    Err(EvalAnswer::Declined(DeclineReason::NotExact))
                                }
                            },
                            answer => Err(answer),
                        }
                    }
                    Err(_) => return FactView::Top(DeclineReason::NotText),
                },
                Part::ParseError(_) => return FactView::Top(DeclineReason::WrongRepresentation),
            };
            match value {
                Ok(value) => bytes.extend_from_slice(&value.bytes),
                Err(EvalAnswer::Pending) => pending = true,
                Err(EvalAnswer::Declined(reason)) => return FactView::Top(reason),
                Err(EvalAnswer::Evaluated(_)) => {
                    return FactView::Top(DeclineReason::Unsupported);
                }
            }
        }
        if pending {
            return FactView::Pending;
        }
        FactView::Exact(exact_of_bytes(bytes), None)
    }
}

/// A variable reference's element-qualified name: the base, or `base(key)`
/// for a constant key; a key that substitutes is a computed name. The one
/// assembly for a substituted word's reads and an expression's quoted
/// operand (`ExprServices::quoted_string`).
pub(crate) fn variable_name(
    reference: &tcl_lexer::word_parts::VarRef<'_>,
) -> Result<String, DeclineReason> {
    use tcl_lexer::word_parts::WordPart as Part;
    let name = std::str::from_utf8(reference.name).map_err(|_| DeclineReason::NotText)?;
    let Some(index) = &reference.index else {
        return Ok(name.to_owned());
    };
    let mut key = Vec::new();
    for piece in index {
        let Part::Text(run) = piece else {
            return Err(DeclineReason::DynamicName);
        };
        key.extend_from_slice(run);
    }
    let key = String::from_utf8(key).map_err(|_| DeclineReason::NotText)?;
    Ok(format!("{name}({key})"))
}

/// A substituted word's structure: its literal runs (decoded), variable
/// reads and script regions, each with its span in the word.
fn word_parts(text: &str, config: LexerConfig) -> Result<Vec<WordPart>, DeclineReason> {
    use tcl_lexer::word_parts::{SubstFlags, WordPart as Part, decompose_spanned};
    let span = |start: usize, end: usize| {
        Span::new(
            u32::try_from(start).unwrap_or(u32::MAX),
            u32::try_from(end).unwrap_or(u32::MAX),
        )
    };
    decompose_spanned(text.as_bytes(), SubstFlags::default(), config)
        .into_iter()
        .map(|spanned| {
            let at = span(spanned.start, spanned.end);
            match spanned.part {
                Part::Text(run) => Ok(WordPart::Literal {
                    span: at,
                    text: String::from_utf8(run.into_owned())
                        .map_err(|_| DeclineReason::NotText)?,
                }),
                Part::Variable(reference) => {
                    let base = std::str::from_utf8(reference.name)
                        .map_err(|_| DeclineReason::NotText)?
                        .to_owned();
                    let element = match variable_name(&reference)? {
                        name if name == base => None,
                        name => Some(name[base.len() + 1..name.len() - 1].to_owned()),
                    };
                    Ok(WordPart::VariableRead {
                        span: at,
                        name: base,
                        element,
                    })
                }
                Part::Command(script) => Ok(WordPart::Script {
                    span: at,
                    script: std::str::from_utf8(script)
                        .map_err(|_| DeclineReason::NotText)?
                        .to_owned(),
                }),
                Part::ParseError(_) => Err(DeclineReason::WrongRepresentation),
            }
        })
        .collect()
}

/// A value's bytes as an exact value, classified as a literal is when they
/// are text.
fn exact_of_bytes(bytes: Vec<u8>) -> ExactValue {
    match std::str::from_utf8(&bytes) {
        Ok(text) => ExactValue::from_literal(text),
        Err(_) => ExactValue {
            bytes,
            numeric: None,
            representation: tcl_registry::value_transfer::RepresentationEvidence::Unknown,
        },
    }
}

/// The bounded-loop simulator's typed `Incr`: the command whose lowering
/// the node is, `name ?amount?` resolved through `registry` and evaluated
/// by the registry's declared route over `env`, the simulator's constant
/// environment. The written value replaces the target's entry; a decline
/// leaves `env` untouched and answers `false`, which ends the simulation.
/// `amount` carries whether its word was braced, as [`LatticeDriver::evaluate_incr`] reads it.
pub(crate) fn exec_cell_update_in_env(
    registry: &CommandRegistry,
    profile: Option<&'static tcl_dialect::DialectProfile>,
    name: &str,
    amount: Option<(&str, bool)>,
    env: &mut crate::static_loops::StaticEnv,
) -> bool {
    let Some(&command) = typed_node_commands(registry, LoweringHookId::Incr).first() else {
        return false;
    };
    let texts: Vec<&str> = std::iter::once(name)
        .chain(amount.map(|(text, _)| text))
        .collect();
    let words: Vec<InvocationWord<'_>> = std::iter::once(InvocationWord::Literal(name))
        .chain(amount.map(amount_word))
        .collect();
    let Some(resolved) = registry
        .resolve_structured_invocation(
            InvocationWords::structured(InvocationWord::Literal(command), &words),
            registry.own_surface_query(),
        )
        .resolved()
    else {
        return false;
    };
    let Some(semantics) = resolved.semantics.value.semantics() else {
        return false;
    };
    let context = AnalysisContext::detached(profile);
    let inputs = EnvInputs {
        view: view_of(&resolved, &texts, &words, InvocationLayout::Source),
        env,
        context: &context,
    };
    let PlanAnswer::CellReadModifyWrite { target, .. } = semantics.structure(&inputs) else {
        return false;
    };
    let route = semantics.route();
    if !matches!(route, EvalRoute::Direct { id } if id.owner() == EvaluatorOwner::Registry) {
        return false;
    }
    let (place, value) = match semantics.evaluate(&inputs, &mut Budget::evaluation()) {
        EvalAnswer::Evaluated(outcome) => {
            match (inputs.place(target.0), written_value(&outcome, target)) {
                (Ok(place), Some(value)) => (place, static_of_exact(value)),
                _ => return false,
            }
        }
        EvalAnswer::Pending | EvalAnswer::Declined(_) => return false,
    };
    env.insert(place.name, value);
    true
}

/// The cell update a `head args…` call resolves to under `registry`, when
/// its declared plan is one: the operation and its target operand. The one
/// question every consumer of the write-chain shape asks (the chain fold,
/// the `append` usage check, the list-length walk), answered from the
/// registry's declaration rather than a command's spelling.
pub(crate) fn resolved_cell_update(
    registry: &CommandRegistry,
    head: &str,
    args: &[String],
) -> Option<(tcl_registry::native_lowering::CellUpdate, OperandId)> {
    let texts: Vec<&str> = args.iter().map(String::as_str).collect();
    let words: Vec<InvocationWord<'_>> = texts.iter().copied().map(word_of).collect();
    let resolved = registry
        .resolve_structured_invocation(
            InvocationWords::structured(InvocationWord::Literal(head), &words),
            registry.own_surface_query(),
        )
        .resolved()?;
    let semantics = resolved.semantics.value.semantics()?;
    let context = AnalysisContext::detached(registry.profile());
    let inputs = StructureInputs {
        view: view_of(&resolved, &texts, &words, InvocationLayout::Source),
        context: &context,
    };
    match semantics.structure(&inputs) {
        PlanAnswer::CellReadModifyWrite {
            target, operation, ..
        } => Some((operation, target.0)),
        _ => None,
    }
}

/// The interval domain's model of a typed cell update with `operands`
/// post-head words: the registry-described operation the domain
/// interprets, from the specialisation the descriptor derives — the typed
/// IR node is that descriptor by construction, so no invocation is
/// resolved and no command is named.
pub(crate) fn cell_update_range_model(
    update: tcl_registry::native_lowering::CellUpdate,
    operands: usize,
) -> Option<tcl_registry::value_transfer::RangeModel> {
    use tcl_registry::value_transfer::{CommandSemantics as _, LiteralInputs};
    let semantics = tcl_registry::value_transfer::cell_update::CellUpdateSemantics {
        update,
        creates_absent: None,
    };
    let args: Vec<&str> = std::iter::repeat_n("", operands).collect();
    let inputs = LiteralInputs::new("", None, &args, None);
    match semantics.transfer(FactDomain::Range, &inputs, &mut Budget::evaluation()) {
        TransferAnswer::Range(model) => Some(model),
        _ => None,
    }
}

/// A simulator value from an exact value: the classification when it has
/// one, else the exact text.
fn static_of_exact(value: &ExactValue) -> crate::static_loops::StaticValue {
    use crate::static_loops::StaticValue;
    match value.numeric {
        Some(NumericValue::Int(i)) => StaticValue::Int(i),
        Some(NumericValue::Float(f)) => StaticValue::Float(f),
        Some(NumericValue::Bool(b)) => StaticValue::Bool(b),
        None => StaticValue::Str(String::from_utf8_lossy(&value.bytes).into_owned()),
    }
}

/// An exact value from a simulator value: the value's canonical text with
/// its classification.
fn exact_of_static(value: &crate::static_loops::StaticValue) -> ExactValue {
    use crate::static_loops::StaticValue;
    match value {
        StaticValue::Int(i) => ExactValue::int(*i),
        StaticValue::Float(f) => ExactValue {
            bytes: f.to_string().into_bytes(),
            numeric: Some(NumericValue::Float(*f)),
            representation: tcl_registry::value_transfer::RepresentationEvidence::Unknown,
        },
        StaticValue::Bool(b) => ExactValue {
            bytes: (if *b { "1" } else { "0" }).as_bytes().to_vec(),
            numeric: Some(NumericValue::Bool(*b)),
            representation: tcl_registry::value_transfer::RepresentationEvidence::Unknown,
        },
        StaticValue::Str(s) => ExactValue::text(s.clone()),
    }
}

/// The analyser's inputs over the bounded-loop simulator's constant
/// environment: a literal word is exact, a `$var` word is the environment's
/// value, and a place's prior value is its entry.
struct EnvInputs<'a> {
    view: ResolvedInvocationView<'a>,
    env: &'a crate::static_loops::StaticEnv,
    context: &'a AnalysisContext,
}

impl AnalysisInputs for EnvInputs<'_> {
    fn invocation(&self) -> &ResolvedInvocationView<'_> {
        &self.view
    }

    fn operand(&self, id: OperandId, domain: FactDomain) -> FactView {
        if domain != FactDomain::ExactValue {
            return FactView::Top(DeclineReason::Unavailable(AnalysisTier::Fast));
        }
        let Some(operand) = self.view.operand(id) else {
            return FactView::Top(DeclineReason::NotExact);
        };
        if let Some(name) = simple_var_ref_name(operand.text) {
            return self.variable(name, domain);
        }
        if operand.text.contains('$') || operand.text.contains('[') {
            return FactView::Top(DeclineReason::NotExact);
        }
        FactView::Exact(ExactValue::from_literal(operand.text), None)
    }

    fn place(&self, id: OperandId) -> Result<PlaceRef, DeclineReason> {
        let text = self
            .view
            .operand(id)
            .map(|operand| operand.text)
            .ok_or(DeclineReason::NotExact)?;
        if text.contains('$') || text.contains('[') {
            return Err(DeclineReason::DynamicName);
        }
        Ok(place_named(crate::naming::normalise_var_name(text)))
    }

    fn variable(&self, name: &str, domain: FactDomain) -> FactView {
        if domain != FactDomain::ExactValue {
            return FactView::Top(DeclineReason::Unavailable(AnalysisTier::Fast));
        }
        self.env
            .get(name)
            .map_or(FactView::Top(DeclineReason::NotExact), |value| {
                FactView::Exact(exact_of_static(value), None)
            })
    }

    fn prior_store(&self, place: &PlaceRef, domain: FactDomain) -> FactView {
        self.variable(&place.name, domain)
    }

    fn word_structure(&self, _id: OperandId) -> Result<WordStructure, DeclineReason> {
        Err(DeclineReason::Unsupported)
    }

    fn body(&self, _id: OperandId) -> Result<BodyRegion, DeclineReason> {
        Err(DeclineReason::Unsupported)
    }

    fn nested(&self, _script: &str, _state: &mut EvaluationState) -> EvalAnswer {
        EvalAnswer::Declined(DeclineReason::Unsupported)
    }

    fn math_function(&self, _name: &str) -> Result<BindingIdentity, DeclineReason> {
        Err(DeclineReason::Unsupported)
    }

    fn context(&self) -> &AnalysisContext {
        self.context
    }
}

/// A place for a normalised name.
fn place_named(name: &str) -> PlaceRef {
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

/// The name a `$var` / `${var}` word reads, or `None` for any other shape.
fn simple_var_ref_name(text: &str) -> Option<&str> {
    // `${a} + ${b}` starts and ends like one braced reference and is not.
    if let Some(name) = text
        .strip_prefix("${")
        .and_then(|s| s.strip_suffix('}'))
        .filter(|name| !name.contains('}'))
    {
        return Some(name);
    }
    let name = text.strip_prefix('$')?;
    name.bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b':')
        .then_some(name)
}

/// Every name the function's calls unbind by literal — the existence
/// fold's unbind fact — from each call's resolved existence transfer.
pub(crate) fn unbound_names(cfg: &CfgFunction, registry: &CommandRegistry) -> FxHashSet<String> {
    let mut out = FxHashSet::default();
    let context = AnalysisContext::detached(registry.profile());
    let mut budget = Budget::unbounded();
    for block in cfg.blocks.values() {
        for stmt in &block.statements {
            let Statement::Call { args, .. } = stmt else {
                continue;
            };
            let head = stmt.canonical_command_or_source();
            let texts: Vec<&str> = args.iter().map(String::as_str).collect();
            let words: Vec<InvocationWord<'_>> = texts.iter().copied().map(word_of).collect();
            let Some(resolved) = registry
                .resolve_structured_invocation(
                    InvocationWords::structured(InvocationWord::Literal(head), &words),
                    registry.own_surface_query(),
                )
                .resolved()
            else {
                continue;
            };
            let Some(semantics) = resolved.semantics.value.semantics() else {
                continue;
            };
            let inputs = StructureInputs {
                view: view_of(&resolved, &texts, &words, InvocationLayout::Source),
                context: &context,
            };
            let TransferAnswer::Existence(transfer) =
                semantics.transfer(FactDomain::Existence, &inputs, &mut budget)
            else {
                continue;
            };
            for path in &transfer.paths {
                for (target, outcome) in &path.outcomes {
                    if *outcome == ExistenceOutcome::Unbind
                        && let Ok(place) = inputs.place(target.0)
                    {
                        out.insert(place.name);
                    }
                }
            }
        }
    }
    out
}

/// Inputs for a structure-only question over source words: literal words
/// are exact, dynamic words are not, and a literal name is a place.
struct StructureInputs<'a> {
    view: ResolvedInvocationView<'a>,
    context: &'a AnalysisContext,
}

impl AnalysisInputs for StructureInputs<'_> {
    fn invocation(&self) -> &ResolvedInvocationView<'_> {
        &self.view
    }

    fn operand(&self, id: OperandId, domain: FactDomain) -> FactView {
        match self.view.operand(id) {
            Some(operand)
                if domain == FactDomain::ExactValue
                    && operand.kind == InvocationWordKind::Literal =>
            {
                FactView::Exact(ExactValue::from_literal(operand.text), None)
            }
            _ => FactView::Top(DeclineReason::NotExact),
        }
    }

    fn place(&self, id: OperandId) -> Result<PlaceRef, DeclineReason> {
        match self.view.operand(id) {
            Some(operand) if operand.kind == InvocationWordKind::Literal => {
                Ok(place_named(operand.text))
            }
            Some(_) => Err(DeclineReason::DynamicName),
            None => Err(DeclineReason::NotExact),
        }
    }

    fn variable(&self, _name: &str, _domain: FactDomain) -> FactView {
        FactView::Top(DeclineReason::Unavailable(AnalysisTier::Structure))
    }

    fn prior_store(&self, _place: &PlaceRef, _domain: FactDomain) -> FactView {
        FactView::Top(DeclineReason::Unavailable(AnalysisTier::Structure))
    }

    fn word_structure(&self, _id: OperandId) -> Result<WordStructure, DeclineReason> {
        Err(DeclineReason::Unsupported)
    }

    fn body(&self, _id: OperandId) -> Result<BodyRegion, DeclineReason> {
        Err(DeclineReason::Unsupported)
    }

    fn nested(&self, _script: &str, _state: &mut EvaluationState) -> EvalAnswer {
        EvalAnswer::Declined(DeclineReason::Unsupported)
    }

    fn math_function(&self, _name: &str) -> Result<BindingIdentity, DeclineReason> {
        Err(DeclineReason::Unsupported)
    }

    fn context(&self) -> &AnalysisContext {
        self.context
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

#[cfg(test)]
mod tests {
    use super::*;

    thread_local! {
        /// The request size the next driver on this thread opens, when set.
        pub(super) static REQUEST_WORK: Cell<Option<u64>> = const { Cell::new(None) };
    }

    /// A run whose evaluations spend its request declines the rest with
    /// `Budget(Request)`, published as `Overdefined`: under the default
    /// request every `string length` of the procedure folds; under one too
    /// small for them, a decline records the request as its reason, and
    /// the join keeps every decline, so no definition the request could
    /// not pay for publishes a value.
    #[test]
    fn an_exhausted_request_declines_the_rest() {
        let body: String = (0..12).fold(String::new(), |mut body, i| {
            use std::fmt::Write as _;
            let _ = writeln!(body, "    set a{i} [string length abc{i}]");
            body
        });
        let source = format!("proc p {{}} {{\n{body}}}\n");
        let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
        let build = || {
            crate::compilation_unit::CompilationUnit::build_for_dialect(
                &source, registry, false, "tcl9.0",
            )
        };
        let value = |unit: &crate::compilation_unit::CompilationUnit, i: usize| {
            let function = unit.procedures.get("::p").expect("the procedure");
            let symbol = function
                .ssa
                .var_symbol(&format!("a{i}"))
                .expect("the variable");
            function.sccp.values.get(&(symbol, 1)).cloned()
        };
        let answers = |unit: &crate::compilation_unit::CompilationUnit| -> Vec<String> {
            unit.procedures
                .get("::p")
                .expect("the procedure")
                .sccp
                .explanations
                .iter()
                .filter(|explanation| explanation.command == "string")
                .map(|explanation| explanation.answer.clone())
                .collect()
        };

        let unit = build();
        for i in 0..12 {
            let expected = if i < 10 { 4 } else { 5 };
            assert_eq!(
                value(&unit, i),
                Some(LatticeValue::Const(ConstValue::Int(expected))),
                "a{i} folds under the default request"
            );
        }
        assert!(answers(&unit).iter().all(|answer| answer == "evaluated"));

        REQUEST_WORK.with(|work| work.set(Some(500)));
        let unit = build();
        REQUEST_WORK.with(|work| work.set(None));
        let answers = answers(&unit);
        assert_eq!(answers.len(), 12, "{answers:?}");
        assert!(
            answers
                .iter()
                .any(|answer| answer == "declined: budget: Request"),
            "{answers:?}"
        );
        for (i, answer) in answers.iter().enumerate() {
            if answer.starts_with("declined") {
                assert_eq!(
                    value(&unit, i),
                    Some(LatticeValue::Overdefined),
                    "a{i}: {answer}"
                );
            }
        }
        assert_eq!(
            value(&unit, 11),
            Some(LatticeValue::Overdefined),
            "the last statement is past what the request pays for"
        );
    }

    #[test]
    fn split_head_basic() {
        assert_eq!(split_head("cmd arg1 arg2"), ("cmd", Some("arg1 arg2")));
        assert_eq!(split_head("  cmd"), ("cmd", None));
        assert_eq!(split_head(""), ("", None));
    }

    /// An expression's answer names every binding it rests on: the `expr`
    /// head, each nested command's head, and each math function (the
    /// wrapper command it dispatches to).
    #[test]
    fn an_expression_answer_names_every_binding_it_rests_on() {
        let registry = CommandRegistry::build_default();
        let mutations = crate::command_binding::ModuleCommandMutations::default();
        let driver = LatticeDriver::detached(
            Some(BuiltinFoldInputs {
                registry: &registry,
                mutations: &mutations,
                dialect: None,
                defining_class: None,
                registry_engine: false,
                trust: crate::sccp::FoldTrust::ObservedBindings,
            }),
            FoldPolicy::default(),
        );
        let node = crate::expr_parser::parse_expr_for_profile("abs([string length abc] - 5)", None);
        let expression = ExpressionEvaluation {
            expression: Expression::Parsed(&node),
            policy: FoldPolicy::default(),
            head: Some(binding_of("expr", "expr")),
        };
        let ssa = SsaFunction::trivial("::p", crate::cfg::BlockId(0), vec!["entry".into()]);
        let uses: HashMap<Symbol, Version> = HashMap::new();
        let values: HashMap<ValueKey, LatticeValue> = HashMap::new();
        let LiftedAnswer::Evaluated(outcomes) =
            driver.evaluate_expression_at(&expression, &uses, &values, &ssa)
        else {
            panic!("the expression evaluates");
        };
        let [outcome] = outcomes.as_slice() else {
            panic!("one outcome");
        };
        assert_eq!(
            outcome.result,
            ExactValueOrUnavailable::Exact(ExactValue::int(2))
        );
        let named: Vec<(&str, &str)> = outcome
            .evidence
            .bindings
            .iter()
            .map(|binding| (binding.name.as_str(), binding.identity.as_str()))
            .collect();
        assert_eq!(
            named,
            [
                ("expr", "expr"),
                ("string", "string"),
                ("abs", "::tcl::mathfunc::abs")
            ]
        );
    }

    #[test]
    fn lattice_constants_round_trip_through_exact_values() {
        for c in [
            ConstValue::Int(42),
            ConstValue::Int(-7),
            ConstValue::Float(1.5),
            ConstValue::Bool(true),
            ConstValue::String(" a ".into()),
            ConstValue::String("a\0b".into()),
            ConstValue::String("a\\b".into()),
            ConstValue::String("08".into()),
        ] {
            assert_eq!(exact_to_const(&const_to_exact(&c)), c, "{c:?}");
        }
    }

    /// D60: `simple_var_ref_name` reads exactly one reference. A lowered
    /// quoted `expr` word spells `${a} + ${b}`, which starts and ends like one
    /// braced reference and is not the variable `a} + ${b`.
    #[test]
    fn a_simple_reference_is_one_reference_only() {
        assert_eq!(simple_var_ref_name("$a"), Some("a"));
        assert_eq!(simple_var_ref_name("${a b}"), Some("a b"));
        assert_eq!(simple_var_ref_name("$::ns::v"), Some("::ns::v"));
        assert_eq!(simple_var_ref_name("${a} + ${b}"), None);
        assert_eq!(simple_var_ref_name("$a + $b"), None);
        assert_eq!(simple_var_ref_name("a"), None);
    }

    /// A condition's truth as `if` reads it: a number is true when non-zero,
    /// a boolean word is its value, and a beyond-wide integer's canonical
    /// spelling is true when non-zero. A digit string with a leading zero is
    /// no canonical spelling: under 8.x it reached the condition as text
    /// because the grammar read it as an invalid octal, and `if {08}` raises
    /// `expected boolean value but got "08"` on tclsh 8.5 and 8.6.
    #[test]
    fn a_condition_reads_only_canonical_digits_as_a_number() {
        let text = |t: &str| ExactValueOrUnavailable::Exact(ExactValue::text(t));
        assert_eq!(truth_of(&text("yes")), Some(true));
        assert_eq!(truth_of(&text("off")), Some(false));
        assert_eq!(truth_of(&text("123456789012345678901234")), Some(true));
        assert_eq!(truth_of(&text("-123456789012345678901234")), Some(true));
        assert_eq!(truth_of(&text("0")), Some(false));
        for undecided in ["08", "-08", "007", "00", "x"] {
            assert_eq!(truth_of(&text(undecided)), None, "{undecided}");
        }
        assert_eq!(
            truth_of(&ExactValueOrUnavailable::Exact(ExactValue::int(0))),
            Some(false)
        );
    }
}
