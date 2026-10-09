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
use tcl_registry::value_transfer::builtins::ExpressionRoute;
use tcl_registry::value_transfer::{
    AnalysisContext, AnalysisInputs, AnalysisTier, BinderName, BindingIdentity, BindingKind,
    BodyRegion, Budget, BudgetLimit, CaseArms, CommandSemantics, CompletionOutcome, DeclineReason,
    DependencyEvidence, DomainFact, EvalAnswer, EvalRoute, EvaluationState, EvaluatorOwner,
    ExactValue, ExactValueOrUnavailable, Existence, ExistenceOutcome, ExitRule, FactBounds,
    FactDomain, FactView, InvocationLayout, InvocationOutcome, IterableKind, IterationPlan,
    LanguageProfileId, LiftedAnswer, LoopStep, NestedPolicy, NumericValue, OperandId, OperandView,
    ParameterDefault, PlaceKind, PlaceRef, PlanAnswer, RepresentationEvidence,
    ResolvedInvocationView, RouteIdentity, SelectionFact, StoreOutcome, TargetId, TargetSemantics,
    TransferAnswer, TypeFacts, ValueIdentity, ValueShape, WordPart, WordStructure, WrittenPlace,
    evaluate_lifted, validate_outcome, written_in,
};
use tcl_registry::{
    ArgRole, CommandRegistry, FrameLevel, InvocationWord, InvocationWordKind, InvocationWords,
    ResolvedInvocation, SemanticOperationId, TclType,
};

use crate::analyses::{ConstValue, LatticeValue, MAX_CONSTSET_SIZE};
use crate::codegen::helpers::split_list_values;
use crate::command_binding::CommandTrustSnapshot;
use crate::expr_ast::ExprNode;
use crate::ir::{CommandTokens, Statement};
use crate::sccp::{
    BuiltinFoldInputs, DefValues, FoldTrust, RaisedDefs, TraceInputs, extract_foreach_elements,
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
    /// The names an iRules `when` handler of the module may find bound on
    /// entry ([`ConnectionScoped`]); empty for a module with no handler.
    pub connection_scoped: ConnectionScoped,
    /// The names the module's callback scripts write
    /// ([`crate::ir::Module::deferred_writes`]): each is externally mutable
    /// in every function, so a callback edited anywhere in the module
    /// re-keys every function's lattice.
    pub deferred_writes: crate::ir::DeferredWrites,
}

/// The names an iRules `when` handler may find bound on entry
/// (`docs/design/compiler/value-transfers.md` § *Existence*, the entry
/// state): a handler's variables live as long as its connection, so another
/// event — or an earlier firing of the same one — may have bound any name a
/// handler of the module binds.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct ConnectionScoped {
    /// Every name a handler of the module binds, and the array each element
    /// among them sits in, sorted and without repeats.
    pub names: Vec<String>,
    /// Whether a handler writes a computed name, so any name may be bound.
    pub any: bool,
}

impl ConnectionScoped {
    /// Whether a handler may find `name` bound on entry.
    #[must_use]
    pub fn holds(&self, name: &str) -> bool {
        self.any
            || self
                .names
                .binary_search_by(|held| held.as_str().cmp(name))
                .is_ok()
    }
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
            connection_scoped: ConnectionScoped::default(),
            deferred_writes: crate::ir::DeferredWrites::default(),
        }
    }

    /// The key with the module's connection-scoped names
    /// ([`Self::connection_scoped`]).
    #[must_use]
    pub fn with_connection_scoped(mut self, scoped: ConnectionScoped) -> Self {
        self.connection_scoped = scoped;
        self
    }

    /// The key with the names the module's callback scripts write
    /// ([`Self::deferred_writes`]).
    #[must_use]
    pub fn with_deferred_writes(mut self, writes: crate::ir::DeferredWrites) -> Self {
        self.deferred_writes = writes;
        self
    }

    /// The key for a request at `tier`. Below the deep tier a function
    /// built under the key runs no existence rung, so the driver answers
    /// each existence read `Unavailable(tier)` and so does
    /// [`crate::compilation_unit::FunctionUnit::existence`].
    #[must_use]
    pub fn at_tier(mut self, tier: AnalysisTier) -> Self {
        self.tier = tier;
        self
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
                StoreOutcome::MayWrite { facts, .. }
                | StoreOutcome::WriteUnavailable { facts, .. } => Self::informative(
                    facts.intrep,
                    facts.shape.clone(),
                    RepresentationEvidence::Unknown,
                ),
                StoreOutcome::Unbind { .. } => None,
                // The type facts state types per target, and the target of
                // an element write is the whole array: only the written
                // value's own evidence speaks for the element.
                StoreOutcome::WriteElement { value, .. } => {
                    Self::informative(None, None, value.representation)
                }
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

/// Each place a call names, with the step its callee's summary takes it by.
type PlaceSteps = [(String, ExistenceStep)];

/// What one call seeds a re-run of its callee with: each argument, exact
/// where the call's words prove it, and each `Name` link's place — its value,
/// where exact, and its existence.
type CallSeeds = (
    Vec<Option<ExactValue>>,
    Vec<(Option<ExactValue>, Existence)>,
);

/// The re-run a call statement asks of its callee.
enum CallRerun {
    /// The re-run, where one can be made.
    Made(Option<std::rc::Rc<crate::interprocedural::Rerun>>),
    /// A word or a place the solver has not reached yet.
    Waiting,
}

/// Each argument word of a call statement as the driver reads it — its
/// literal value, `None` for one that substitutes — or `None` where a word
/// expands, so no word's position is known.
pub(crate) fn call_literal_words(
    args: &[String],
    tokens: Option<&CommandTokens>,
    config: &LexerConfig,
) -> Option<Vec<Option<String>>> {
    let cooked = call_arguments(args, tokens, config);
    if cooked
        .iter()
        .any(|word| word.kind == InvocationWordKind::Expanded)
    {
        return None;
    }
    Some(
        literal_words(&cooked)
            .into_iter()
            .map(|word| word.map(str::to_owned))
            .collect(),
    )
}

/// The step each place a call statement's `Name` arguments name takes from
/// its callee's transfer summary, the callee resolved from `function` under
/// the shared lattice's trust: what the read-before-set check reads of the
/// call. Empty for any other statement, and for a call the summary does not
/// answer or that never completes normally.
pub(crate) fn summary_steps(
    module: &crate::interprocedural::ModuleProcedures<'_>,
    function: &str,
    statement: &Statement,
    config: &LexerConfig,
) -> Vec<(String, ExistenceStep)> {
    let Statement::Call {
        command,
        args,
        tokens,
        foreach_groups: None,
        ..
    } = statement
    else {
        return Vec::new();
    };
    if statement.synthetic_marker().is_some() {
        return Vec::new();
    }
    let Some(callee) = module.resolve(command, function, FoldTrust::ObservedBindings) else {
        return Vec::new();
    };
    let Some(words) = call_literal_words(args, tokens.as_ref(), config) else {
        return Vec::new();
    };
    let words: Vec<Option<&str>> = words.iter().map(Option::as_deref).collect();
    match module.call_transfer(&callee, &words) {
        Some(crate::interprocedural::CallTransfer::Places(places)) => places,
        _ => Vec::new(),
    }
}

/// Each word's literal text, `None` for a word that substitutes.
fn literal_words<'w>(cooked: &'w [ArgWord<'_>]) -> Vec<Option<&'w str>> {
    cooked
        .iter()
        .map(|word| (word.kind == InvocationWordKind::Literal).then_some(word.text.as_ref()))
        .collect()
}

/// The answer for each definition of a call statement to a procedure of the
/// module: a place a `Name` argument names takes the fact and the value the
/// re-run leaves in it, where one was made, else the summary's step and no
/// value; an outer place the callee may write takes the summary's step; and
/// any other definition is left may-bound with no value.
fn place_answers(
    defs: &[(String, ValueKey)],
    (places, outer): (&PlaceSteps, &PlaceSteps),
    after: Option<&crate::interprocedural::Rerun>,
) -> Vec<DefAnswer> {
    defs.iter()
        .map(|(name, key)| {
            if let Some(slot) = places.iter().position(|(place, _)| place == name) {
                let left = after.and_then(|after| after.places.get(slot));
                let existence = left
                    .and_then(|left| left.existence)
                    .map_or(places[slot].1, ExistenceStep::Set);
                let value = left
                    .filter(|left| matches!(left.existence, Some(Existence::Bound(_))))
                    .and_then(|left| left.value.as_ref())
                    .map_or(LatticeValue::Overdefined, exact_to_lattice);
                return DefAnswer {
                    stated: true,
                    existence,
                    ..DefAnswer::untyped(*key, value)
                };
            }
            let existence = outer
                .iter()
                .find(|(place, _)| place == name)
                .map_or(ExistenceStep::UNKNOWN, |(_, step)| *step);
            DefAnswer {
                existence,
                ..DefAnswer::untyped(*key, LatticeValue::Overdefined)
            }
        })
        .collect()
}

/// The view a `[…]` command to the procedure `callee` presents its words
/// through: each operand's text and kind, with no registry role.
fn procedure_view<'a>(
    callee: &'a str,
    texts: &[&'a str],
    cooked: &[ArgWord<'_>],
) -> ResolvedInvocationView<'a> {
    ResolvedInvocationView {
        canonical_command: callee,
        subcommand: None,
        form: None,
        layout: InvocationLayout::Source,
        operands: texts
            .iter()
            .zip(cooked)
            .map(|(text, word)| OperandView {
                text,
                kind: word.kind,
                role: None,
            })
            .collect(),
        argument_offset: 0,
        arity: None,
    }
}

/// The writes a `[…]` command answers under `policy`: none where it runs
/// effect-free, else `writes` as its one member.
fn member_writes(
    policy: NestedPolicy,
    writes: Vec<(PlaceRef, StoreOutcome)>,
) -> Vec<Vec<(PlaceRef, StoreOutcome)>> {
    if policy == NestedPolicy::EffectFreeOnly {
        Vec::new()
    } else {
        vec![writes]
    }
}

/// A `[…]` command to a procedure of the module that never completes
/// normally: an error before any store, resting on the procedure's binding.
fn procedure_raises(binding: &BindingIdentity) -> LiftedAnswer {
    let mut evidence = DependencyEvidence::default();
    record_binding(&mut evidence, binding.clone());
    LiftedAnswer::Evaluated(vec![Box::new(InvocationOutcome {
        completion: CompletionOutcome::error_unproven(0),
        result: ExactValueOrUnavailable::unproven_string(),
        nested_writes: Vec::new(),
        ordered_stores: Vec::new(),
        types: TypeFacts::default(),
        evidence,
    })])
}

/// The stores a `[…]` command to a procedure of the module makes: each place
/// a `Name` argument names takes what the re-run leaves in it, where one was
/// made, else the summary's step with no value, and each outer place the
/// callee may write takes the summary's step.
fn place_stores(
    (places, links): (&PlaceSteps, &[(usize, String)]),
    outer: &PlaceSteps,
    after: Option<&crate::interprocedural::Rerun>,
) -> Vec<(PlaceRef, StoreOutcome)> {
    let mut stores = Vec::new();
    for (slot, ((name, step), (param, _))) in places.iter().zip(links).enumerate() {
        let target = TargetId(OperandId(*param));
        let left = after.and_then(|after| after.places.get(slot));
        let made = match (
            left.and_then(|left| left.existence),
            left.and_then(|left| left.value.clone()),
        ) {
            (Some(Existence::Bound(_)), Some(value)) => vec![StoreOutcome::Write { target, value }],
            (Some(Existence::Bound(kind)), None) => vec![StoreOutcome::WriteUnavailable {
                target,
                facts: bound_to(kind),
            }],
            (Some(Existence::Unbound), _) => vec![StoreOutcome::Unbind { target }],
            _ => stores_of(*step, target),
        };
        stores.extend(made.into_iter().map(|store| (place_named(name), store)));
    }
    for (name, step) in outer {
        stores.extend(
            stores_of(*step, TargetId(OperandId(0)))
                .into_iter()
                .map(|store| (place_named(name), store)),
        );
    }
    stores
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

/// The stores that take a place as `step` says, with no value: a binding a
/// write whose value is unavailable, an unbinding, a preserve, a may-binding
/// a may-write, and a place left may-bound whatever it held an unbinding
/// then a may-write.
fn stores_of(step: ExistenceStep, target: TargetId) -> Vec<StoreOutcome> {
    match step {
        ExistenceStep::Set(Existence::Bound(kind)) => vec![StoreOutcome::WriteUnavailable {
            target,
            facts: bound_to(kind),
        }],
        ExistenceStep::Set(Existence::Unbound) => vec![StoreOutcome::Unbind { target }],
        ExistenceStep::Join(Existence::Bound(kind)) => vec![StoreOutcome::MayWrite {
            target,
            facts: bound_to(kind),
        }],
        ExistenceStep::Set(_) => vec![
            StoreOutcome::Unbind { target },
            StoreOutcome::MayWrite {
                target,
                facts: bound_to(BindingKind::Either),
            },
        ],
        ExistenceStep::Join(_) => vec![StoreOutcome::Preserve { target }],
    }
}

/// Every definition in `defs` widened: the answer for a statement whose
/// evaluation declined.
fn widened(defs: &[(String, ValueKey)]) -> Vec<DefAnswer> {
    defs.iter()
        .map(|(_, key)| DefAnswer::untyped(*key, LatticeValue::Overdefined))
        .collect()
}

/// The existence step one store takes on its place: a write binds a
/// scalar (an element is one), an unbind unbinds, a preserve keeps the
/// fact, and a may-write joins it with the kind its bounds state.
fn store_existence(store: &StoreOutcome) -> ExistenceStep {
    match store {
        StoreOutcome::Write { .. } | StoreOutcome::WriteElement { .. } => {
            ExistenceStep::Set(Existence::Bound(BindingKind::Scalar))
        }
        StoreOutcome::WriteUnavailable { facts, .. } => {
            ExistenceStep::Set(Existence::Bound(match facts.existence {
                Existence::Bound(kind) => kind,
                Existence::Pending | Existence::Unbound | Existence::MayBound => {
                    BindingKind::Either
                }
            }))
        }
        StoreOutcome::Preserve { .. } => ExistenceStep::PRESERVE,
        StoreOutcome::Unbind { .. } => ExistenceStep::Set(Existence::Unbound),
        StoreOutcome::MayWrite { facts, .. } => ExistenceStep::Join(Existence::Bound(match facts
            .existence
        {
            Existence::Bound(kind) => kind,
            Existence::Pending | Existence::Unbound | Existence::MayBound => BindingKind::Either,
        })),
    }
}

/// The existence step a definition of `name` takes from an outcome's
/// ordered steps per place: its own place's steps in execution order; for
/// the array that holds an element the outcome writes, unbinds or may
/// write, an array binding (an element exists only in an array, and
/// unsetting one leaves the array); for an element of an array the outcome
/// unbinds whole, an unbinding. `None` when no step names the definition.
fn existence_from(steps: &[(PlaceRef, ExistenceStep)], name: &str) -> Option<ExistenceStep> {
    let own = steps
        .iter()
        .filter(|(place, _)| place.name == name)
        .map(|(_, step)| *step)
        .reduce(ExistenceStep::then);
    if own.is_some() {
        return own;
    }
    let array = steps
        .iter()
        .filter(|(place, _)| place.is_element() && place.base() == name)
        .filter_map(|(_, step)| match step {
            ExistenceStep::Set(_) => Some(ExistenceStep::Set(Existence::Bound(BindingKind::Array))),
            ExistenceStep::Join(Existence::Pending) => None,
            ExistenceStep::Join(_) => {
                Some(ExistenceStep::Join(Existence::Bound(BindingKind::Array)))
            }
        })
        .reduce(ExistenceStep::then);
    if array.is_some() {
        return array;
    }
    let base = name
        .split_once('(')
        .filter(|_| name.ends_with(')'))
        .map(|(base, _)| base)?;
    steps
        .iter()
        .any(|(place, step)| place.name == base && *step == ExistenceStep::Set(Existence::Unbound))
        .then_some(ExistenceStep::Set(Existence::Unbound))
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

/// The completion an evaluated outcome has, for an explanation: nothing for
/// the normal one, which is what `evaluated` says, and the code or the
/// number of stores that ran before the error otherwise.
fn completion_label(completion: &CompletionOutcome) -> Option<String> {
    match PathCompletion::of(completion) {
        PathCompletion::Normal => None,
        other => Some(other.label()),
    }
}

/// One store, as an explanation spells it.
fn describe_store(name: &str, store: &StoreOutcome) -> String {
    let text = |value: &ExactValue| String::from_utf8_lossy(&value.bytes).into_owned();
    match store {
        StoreOutcome::Write { value, .. } => format!("write {name} = {}", text(value)),
        StoreOutcome::WriteElement { key, value, .. } => {
            format!("write {name}({key}) = {}", text(value))
        }
        StoreOutcome::Preserve { .. } => format!("preserve {name}"),
        StoreOutcome::Unbind { .. } => format!("unbind {name}"),
        StoreOutcome::MayWrite { .. } => format!("may-write {name}"),
        StoreOutcome::WriteUnavailable { .. } => format!("write {name} = (unavailable)"),
    }
}

/// What each outcome of an evaluated answer stores on its completion path,
/// for an explanation: the stores that ran, in execution order, a place named
/// as the invocation resolved it. Nothing is listed for a normal completion
/// that stores nothing.
fn outcome_paths(
    outcomes: &[Box<InvocationOutcome>],
    input: Option<&dyn AnalysisInputs>,
) -> Vec<PathExplanation> {
    if outcomes.iter().all(|outcome| {
        outcome.completion.is_normal()
            && outcome.nested_writes.is_empty()
            && outcome.ordered_stores.is_empty()
    }) {
        return Vec::new();
    }
    outcomes
        .iter()
        .map(|outcome| {
            let ran = match outcome.completion {
                CompletionOutcome::Error { written, .. } => written,
                _ => usize::MAX,
            };
            let nested = outcome
                .nested_writes
                .iter()
                .map(|(place, store)| describe_store(&place.name, store));
            let ordered = outcome.ordered_stores.iter().map(|store| {
                let target = store.target().0;
                let name = input
                    .and_then(|input| input.place(target).ok())
                    .map_or_else(|| format!("operand {}", target.0), |place| place.name);
                describe_store(&name, store)
            });
            PathExplanation {
                completion: PathCompletion::of(&outcome.completion),
                stores: nested.chain(ordered).take(ran).collect(),
            }
        })
        .collect()
}

/// The paths an existence transfer lists, for an explanation: how each
/// completes and what it does to each target's existence.
fn transfer_paths(
    transfer: &tcl_registry::value_transfer::ExistenceTransfer,
    input: &dyn AnalysisInputs,
) -> Vec<PathExplanation> {
    transfer
        .paths
        .iter()
        .map(|path| {
            let stores = path
                .outcomes
                .iter()
                .map(|(target, outcome)| {
                    let name = input
                        .place(target.0)
                        .map_or_else(|_| format!("operand {}", target.0.0), |place| place.name);
                    let kind = |kind: BindingKind| match kind {
                        BindingKind::Scalar => "scalar",
                        BindingKind::Array => "array",
                        BindingKind::Either => "either",
                    };
                    match outcome {
                        ExistenceOutcome::Bind(bound) => format!("bind {name} as {}", kind(*bound)),
                        ExistenceOutcome::Unbind => format!("unbind {name}"),
                        ExistenceOutcome::Preserve => format!("preserve {name}"),
                        ExistenceOutcome::MayBind(bound) => {
                            format!("may-bind {name} as {}", kind(*bound))
                        }
                    }
                })
                .collect();
            PathExplanation {
                completion: PathCompletion::listed(path.completion),
                stores,
            }
        })
        .collect()
}

/// The lifted answer's spelling for an explanation.
fn answer_label(answer: &LiftedAnswer) -> String {
    match answer {
        LiftedAnswer::Pending => "pending".to_owned(),
        LiftedAnswer::Declined(reason) => format!("declined: {}", reason_label(*reason)),
        LiftedAnswer::Evaluated(outcomes) if outcomes.len() == 1 => {
            completion_label(&outcomes[0].completion).map_or_else(
                || "evaluated".to_owned(),
                |completion| format!("evaluated: {completion}"),
            )
        }
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
            reads_module: Cell::new(false),
        }
    }

    /// Say which function the run analyses and what it reads of the module
    /// ([`crate::sccp::ModuleRun`]). The module's summary revision rides the
    /// context's seeds revision.
    pub(crate) fn in_module(mut self, function: &'a str, run: crate::sccp::ModuleRun<'a>) -> Self {
        self.function = function;
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
    pub(crate) fn parameter_default(&self, procedure: &str, parameter: &str) -> ParameterDefault {
        self.reads_module.set(true);
        self.module.map_or(ParameterDefault::Unknown, |module| {
            module.parameter_default(
                self.function,
                (procedure, parameter),
                self.procedure_trust(),
            )
        })
    }

    /// The module's procedures for a call whose head is `head`, where the
    /// run holds them. A run that holds none records that it would have read
    /// them where `head` may name a procedure a transfer summary answers for
    /// ([`crate::sccp::SccpResult::reads_module`]).
    fn procedures_for(
        &self,
        head: &str,
    ) -> Option<&'a crate::interprocedural::ModuleProcedures<'a>> {
        if self.module.is_none() && self.may_name_a_summary(head) {
            self.reads_module.set(true);
        }
        self.module
    }

    /// Whether `head` may name a procedure of the module a transfer summary
    /// answers for, as a run that holds no module can tell: the module has
    /// summaries only at the deep tier and where it rebinds no builtin,
    /// and a registry command whose binding the module leaves
    /// standing names none.
    fn may_name_a_summary(&self, head: &str) -> bool {
        let Some(folds) = self.folds else {
            return true;
        };
        self.context.tier == AnalysisTier::Deep
            && !folds.mutations.rebinds_builtins()
            && (!self.trusted(head)
                || self
                    .registry
                    .get(head.strip_prefix("::").unwrap_or(head))
                    .is_none())
    }

    /// The stance a re-run this run asks for is solved under: its own.
    fn rerun_stance(&self) -> Option<crate::interprocedural::RerunStance<'a>> {
        Some(crate::interprocedural::RerunStance {
            policy: self.policy,
            folds: self.folds?,
        })
    }

    /// A call statement to a procedure of the module, applied as the
    /// callee's transfer summary says (§ *Proc-level transfer summaries*):
    /// each place a `Name` argument names, and each outer place the callee
    /// may write, takes the existence the summary states; and, but in a
    /// summary's own run, a place takes the fact and the value a re-run of
    /// the callee leaves in it — its parameters holding the call's arguments
    /// and its links the places' facts before the call — where the re-run
    /// can be made. A call that never completes normally — a word count the
    /// callee's parameters reject, a callee that never completes, a re-run
    /// under the call's seeds that reaches no exit — is a certain raise
    /// where the run reads how the procedure completes
    /// ([`Self::never_completes`]), and a call whose completion no re-run
    /// decided is recorded there. `None` leaves the call to the generic
    /// answer: a head naming no procedure of the module whose binding
    /// stands, a callee whose summary does not answer the call, a call that
    /// defines nothing and may complete, and a statement a throw leaves
    /// from, whose partial writes no summary states.
    pub(crate) fn procedure_answer<S: std::hash::BuildHasher>(
        &self,
        (block, index): (&crate::ssa::SsaBlock, usize),
        values: &HashMap<ValueKey, LatticeValue, S>,
        ssa: &SsaFunction,
    ) -> Option<DefValues> {
        let stmt_ssa = block.statements.get(index)?;
        let Statement::Call {
            command,
            args,
            tokens,
            foreach_groups: None,
            ..
        } = &stmt_ssa.statement
        else {
            return None;
        };
        if (stmt_ssa.defs.is_empty() && !self.reads_exits)
            || stmt_ssa.statement.synthetic_marker().is_some()
            || self.is_throwing()
        {
            return None;
        }
        let module = self.procedures_for(command)?;
        let callee = module.resolve(command, self.function, self.procedure_trust())?;
        self.reads_module.set(true);
        let defs = named_defs(stmt_ssa, ssa);
        let cooked = call_arguments(args, tokens.as_ref(), &self.lexer_config);
        if cooked
            .iter()
            .any(|word| word.kind == InvocationWordKind::Expanded)
        {
            if !defs.is_empty() {
                self.record_undecided();
            }
            return None;
        }
        let words = literal_words(&cooked);
        let places = match module.call_transfer(&callee, &words) {
            Some(crate::interprocedural::CallTransfer::Never) => {
                return Some(self.never_completes(&defs));
            }
            _ if defs.is_empty() => return None,
            Some(crate::interprocedural::CallTransfer::Places(places)) => places,
            None => {
                self.record_undecided();
                return None;
            }
        };
        let after = if self.level == crate::sccp::ModuleLevel::Outcomes {
            None
        } else {
            let Some(rerun) = self.statement_rerun(
                (module, &callee),
                (block, index),
                (&cooked, &places),
                (values, ssa),
            ) else {
                self.record_undecided();
                return None;
            };
            match rerun {
                CallRerun::Made(after) => after,
                CallRerun::Waiting => {
                    return Some(DefValues::PerDef(waiting(&defs, &LatticeValue::Unknown)));
                }
            }
        };
        match after.as_deref() {
            Some(after) if !after.completes => return Some(self.never_completes(&defs)),
            Some(after) if after.decided => {}
            _ if self.level != crate::sccp::ModuleLevel::Outcomes => self.record_undecided(),
            _ => {}
        }
        Some(DefValues::PerDef(place_answers(
            &defs,
            (&places, &module.outer_steps(&callee)),
            after.as_deref(),
        )))
    }

    /// A call statement that never completes normally: where a raise ends
    /// the path a certain raise before any store, so no exit past it is
    /// read; elsewhere each definition waits on an existence the call never
    /// gives, and no code after the call is proved unreachable by it.
    fn never_completes(&self, defs: &[(String, ValueKey)]) -> DefValues {
        if self.raises_end_paths() {
            DefValues::Raised(Box::new(RaisedDefs {
                prefix: None,
                written: 0,
            }))
        } else {
            DefValues::PerDef(waiting(defs, &LatticeValue::Overdefined))
        }
    }

    /// The re-run of `callee` a call statement asks for: its parameters
    /// holding the statement's words, exact where the solver proves them,
    /// and its links the places the call names (`places`, in the callee's
    /// link order), as the solver holds them before the call. `None` where
    /// a place is not a variable of the frame the call spells.
    fn statement_rerun<S: std::hash::BuildHasher>(
        &self,
        (module, callee): (&crate::interprocedural::ModuleProcedures<'a>, &str),
        (block, index): (&crate::ssa::SsaBlock, usize),
        (cooked, places): (&[ArgWord<'_>], &PlaceSteps),
        (values, ssa): (&HashMap<ValueKey, LatticeValue, S>, &SsaFunction),
    ) -> Option<CallRerun> {
        let inputs = self.expression_inputs((&block.statements.get(index)?.uses, values, ssa));
        let mut arguments = Vec::with_capacity(cooked.len());
        for word in cooked {
            arguments.push(match word.kind {
                InvocationWordKind::Literal => Some(ExactValue::from_literal(&word.text)),
                _ if word.source == OperandSource::Substituted => {
                    match inputs.substituted(&word.text) {
                        FactView::Exact(value, _) => Some(value),
                        FactView::Pending => return Some(CallRerun::Waiting),
                        _ => None,
                    }
                }
                _ => None,
            });
        }
        let mut seeds = Vec::new();
        for (name, _) in places {
            let symbol = ssa.var_symbol(name)?;
            let prior = crate::sccp::prior_version(block, index, symbol);
            let value = match values.get(&(symbol, prior)) {
                Some(LatticeValue::Unknown) => return Some(CallRerun::Waiting),
                Some(LatticeValue::Const(value)) => Some(const_to_exact(value)),
                _ => None,
            };
            // With no existence rung, a procedure's entry version of a name
            // is bound for a parameter and unbound for any other.
            let entry = module
                .params_of(self.function)
                .filter(|_| prior == 0)
                .map(|params| {
                    if params.iter().any(|param| param == name) {
                        Existence::Bound(BindingKind::Scalar)
                    } else {
                        Existence::Unbound
                    }
                });
            let existence = match (self.existence_now(symbol), entry) {
                (Some(Existence::Pending), _) => return Some(CallRerun::Waiting),
                (Some(fact), _) | (None, Some(fact)) => fact,
                (None, None) if value.is_some() => Existence::Bound(BindingKind::Scalar),
                (None, None) => Existence::MayBound,
            };
            seeds.push((value, existence));
        }
        Some(CallRerun::Made(self.rerun_with(
            (module, callee),
            &arguments,
            &seeds,
        )))
    }

    /// `callee` re-run under this run's stance, its parameters holding
    /// `arguments` — an omitted one its default — and its links `seeds`.
    fn rerun_with(
        &self,
        (module, callee): (&crate::interprocedural::ModuleProcedures<'a>, &str),
        arguments: &[Option<ExactValue>],
        seeds: &[(Option<ExactValue>, Existence)],
    ) -> Option<std::rc::Rc<crate::interprocedural::Rerun>> {
        let dialect = self.folds.and_then(|folds| folds.dialect);
        module
            .parameter_values(callee, arguments, dialect)
            .zip(self.rerun_stance())
            .and_then(|(params, stance)| module.rerun(callee, (&params, seeds), stance))
    }

    /// One `[…]` command naming a procedure of the module, run as the
    /// callee's transfer summary says: its words first, then a write to each
    /// place a `Name` argument names — the value a re-run of the callee
    /// leaves there where one is made, else what the summary bounds — and
    /// its result where the run takes results ([`crate::sccp::ModuleLevel::
    /// Results`]). A command that never completes normally raises before it
    /// stores, where a raise ends the path, and one whose completion no
    /// re-run decided is recorded there. `None` leaves the command to the
    /// registry: a head naming no procedure of the module; below `Results`,
    /// a command whose writes nothing would carry (the effect-free policy);
    /// and a protected script or a statement a throw leaves from, since a
    /// summary says how a call may complete, not that it completes normally.
    fn procedure_run<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &self,
        seg: &crate::segmenter::SegmentedCommand,
        (uses, values, ssa): Lattice<'_, S1, S2>,
        (prior, policy): (&[(PlaceRef, StoreOutcome)], NestedPolicy),
    ) -> Option<ScriptRun> {
        let head = seg.name();
        if self.leaves_procedure_run(head, policy) {
            return None;
        }
        let module = self.procedures_for(head)?;
        let callee = module.resolve(head, self.function, self.procedure_trust())?;
        self.reads_module.set(true);
        let binding = binding_of(head, &callee);
        let answered = |answer, writes| {
            Some(ScriptRun {
                head: head.to_owned(),
                route: None,
                answer,
                writes,
                binding: binding.clone(),
                rebound: false,
                procedure: true,
            })
        };
        let declined = |reason| answered(LiftedAnswer::Declined(reason), Vec::new());
        // A command that never completes normally raises before it stores
        // anything, where a raise ends the path.
        let raises = || {
            answered(
                procedure_raises(&binding),
                member_writes(policy, Vec::new()),
            )
        };
        let cooked = self.cooked_args(seg);
        if cooked
            .iter()
            .any(|word| word.kind == InvocationWordKind::Expanded)
        {
            self.record_undecided();
            return None;
        }
        let words = literal_words(&cooked);
        let places = match module.call_transfer(&callee, &words) {
            Some(crate::interprocedural::CallTransfer::Places(places)) => places,
            Some(crate::interprocedural::CallTransfer::Never) if self.raises_end_paths() => {
                return raises();
            }
            _ => {
                self.record_undecided();
                return None;
            }
        };
        let outer = module.outer_steps(&callee);
        if policy == NestedPolicy::EffectFreeOnly && (!places.is_empty() || !outer.is_empty()) {
            self.record_undecided();
            return declined(DeclineReason::StatefulNested);
        }
        let texts: Vec<&str> = cooked.iter().map(|arg| arg.text.as_ref()).collect();
        let inputs = LatticeInputs {
            driver: self,
            prior_writes: prior.to_vec(),
            words: Words::under(policy),
            view: procedure_view(&callee, &texts, &cooked),
            uses,
            values,
            ssa,
            sources: cooked.iter().map(|arg| arg.source).collect(),
        };
        let links = module.links(&callee)?;
        let (arguments, seeds) = match self.run_seeds(&inputs, (words.len(), &places), ssa) {
            Ok(seeds) => seeds,
            Err(answer) => return answered(answer, Vec::new()),
        };
        let after = self.rerun_with((module, &callee), &arguments, &seeds);
        match after.as_deref() {
            Some(after) if !after.completes => {
                if self.raises_end_paths() {
                    return raises();
                }
                return declined(DeclineReason::Unsupported);
            }
            Some(after) if after.decided => {}
            _ => self.record_undecided(),
        }
        let mut writes = inputs.words.writes();
        writes.extend(place_stores((&places, &links), &outer, after.as_deref()));
        answered(
            self.procedure_outcome(after.as_deref(), &binding),
            member_writes(policy, writes),
        )
    }

    /// Whether [`Self::procedure_run`] leaves a `[…]` command headed `head`
    /// to the registry before resolving it: in a protected script or a
    /// statement a throw leaves from, and, below `Results`, where its writes
    /// nothing would carry (the effect-free policy) — there a call to a
    /// procedure of the module runs nowhere, so its completion is one no
    /// re-run decided.
    fn leaves_procedure_run(&self, head: &str, policy: NestedPolicy) -> bool {
        if policy == NestedPolicy::Protected || self.is_throwing() {
            return true;
        }
        if policy != NestedPolicy::EffectFreeOnly || self.level == crate::sccp::ModuleLevel::Results
        {
            return false;
        }
        if self.module.is_some_and(|module| {
            module
                .resolve(head, self.function, self.procedure_trust())
                .is_some()
        }) {
            self.record_undecided();
        }
        true
    }

    /// A `[…]` command to a procedure of the module that ran: it completes
    /// normally, with the result the re-run proves where the run takes
    /// results, and otherwise one the source does not give.
    fn procedure_outcome(
        &self,
        after: Option<&crate::interprocedural::Rerun>,
        binding: &BindingIdentity,
    ) -> LiftedAnswer {
        let result = match (after, self.level) {
            (Some(after), crate::sccp::ModuleLevel::Results) => after.result.clone().map_or_else(
                || ExactValueOrUnavailable::Unavailable(bound_to(BindingKind::Scalar)),
                ExactValueOrUnavailable::Exact,
            ),
            _ => ExactValueOrUnavailable::Unavailable(bound_to(BindingKind::Scalar)),
        };
        let mut evidence = DependencyEvidence::default();
        record_binding(&mut evidence, binding.clone());
        LiftedAnswer::Evaluated(vec![Box::new(InvocationOutcome {
            completion: CompletionOutcome::Normal,
            result,
            nested_writes: Vec::new(),
            ordered_stores: Vec::new(),
            types: TypeFacts::default(),
            evidence,
        })])
    }

    /// The arguments and the `Name` places a `[…]` command's re-run is
    /// seeded with, as its words leave them: every operand of the `count`
    /// first, the substituting ones evaluated in order, then each place the
    /// call names (`places`, in the callee's link order) after their
    /// writes. `Err` with the command's answer where a word or a place waits
    /// on the solver.
    fn run_seeds<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &self,
        inputs: &LatticeInputs<'_, S1, S2>,
        (count, places): (usize, &PlaceSteps),
        ssa: &SsaFunction,
    ) -> Result<CallSeeds, LiftedAnswer> {
        let mut arguments = Vec::with_capacity(count);
        for index in 0..count {
            arguments.push(
                match inputs.operand(OperandId(index), FactDomain::ExactValue) {
                    FactView::Exact(value, _) => Some(value),
                    FactView::Pending => return Err(LiftedAnswer::Pending),
                    _ => None,
                },
            );
        }
        let mut seeds = Vec::with_capacity(places.len());
        for (name, _) in places {
            let name = name.as_str();
            let value = match inputs.words_written(name) {
                WrittenPlace::Exact(value) => FactView::Exact(value, None),
                WrittenPlace::Unknown => FactView::Top(DeclineReason::NotExact),
                WrittenPlace::Untouched => inputs.named_fact(name),
            };
            let value = match value {
                FactView::Exact(value, _) => Some(value),
                FactView::Pending => return Err(LiftedAnswer::Pending),
                _ => None,
            };
            let existence = match inputs.written_existence(name) {
                Some(FactView::Domain(DomainFact::Existence(fact))) => fact,
                Some(_) => Existence::MayBound,
                None => match self.existence_fact(ssa, name) {
                    FactView::Domain(DomainFact::Existence(Existence::Pending)) => {
                        return Err(LiftedAnswer::Pending);
                    }
                    FactView::Domain(DomainFact::Existence(fact)) => fact,
                    _ if value.is_some() => Existence::Bound(BindingKind::Scalar),
                    _ => Existence::MayBound,
                },
            };
            seeds.push((value, existence));
        }
        Ok((arguments, seeds))
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

    /// Say whether the statements about to be evaluated sit where a throw
    /// leaves from.
    pub(crate) fn set_throwing(&self, throwing: bool) {
        self.throwing.set(throwing);
    }

    /// Whether the statement being evaluated sits where a throw leaves
    /// from.
    pub(crate) fn is_throwing(&self) -> bool {
        self.throwing.get()
    }

    /// Whether a statement that certainly raises ends its path where the
    /// statement being evaluated sits: where a throw leaves from, and
    /// anywhere in a run that reads how the procedure completes.
    pub(crate) fn raises_end_paths(&self) -> bool {
        self.throwing.get() || self.reads_exits
    }

    /// Open a solver sweep's record of the calls whose completion no
    /// re-run decided: the settled sweep's answer is the run's.
    pub(crate) fn reset_undecided_for_sweep(&self) {
        self.undecided.set(false);
    }

    /// Record that a call the run applied has a completion no re-run
    /// decided, in a run that reads how the procedure completes.
    fn record_undecided(&self) {
        if self.reads_exits {
            self.undecided.set(true);
        }
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
                paths: Vec::new(),
            },
        );
    }

    /// Attach the completion paths the statement being evaluated was found
    /// to have to the explanation recorded for it.
    fn explain_paths(&self, paths: Vec<PathExplanation>) {
        let Some(span) = self.explaining.get() else {
            return;
        };
        if let Some(explanation) = self
            .explanations
            .borrow_mut()
            .get_mut(&(span.start(), span.end()))
        {
            explanation.paths = paths;
        }
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

    /// Record whether the latest evaluation of `key`'s statement preserved
    /// its place, and the version it preserved; the settled sweep's answer
    /// stays, as for [`Self::record_folded`]. `None` removes an earlier
    /// sweep's.
    pub(crate) fn record_preserved(&self, key: ValueKey, prior: Option<Version>) {
        let mut map = self.preserved.borrow_mut();
        match prior {
            Some(prior) => {
                map.insert(key, prior);
            }
            None => {
                map.remove(&key);
            }
        }
    }

    /// Every preserved definition the run holds at its end.
    pub(crate) fn take_preserved(&self) -> HashMap<ValueKey, Version> {
        std::mem::take(&mut *self.preserved.borrow_mut())
    }

    /// Record whether the latest evaluation of the statement at `site`, a
    /// `(block, index)`, certainly raised where a handler is thrown to; the
    /// settled sweep's answer stays, as for [`Self::record_preserved`].
    pub(crate) fn record_raised(&self, site: (crate::cfg::BlockId, usize), raised: bool) {
        let mut set = self.raised.borrow_mut();
        let mut uncaught = self.uncaught.borrow_mut();
        if raised {
            set.insert(site);
            if !self.throwing.get() {
                uncaught.insert(site);
            }
        } else {
            set.remove(&site);
            uncaught.remove(&site);
        }
    }

    /// Every statement the run's last sweep proved raises.
    pub(crate) fn take_raised(&self) -> HashSet<(crate::cfg::BlockId, usize)> {
        std::mem::take(&mut *self.raised.borrow_mut())
    }

    /// Position the existence rung at a block's entry: `state` holds one
    /// fact per symbol, and the statements the solver evaluates next read
    /// and advance it.
    pub(crate) fn existence_enter(&self, state: Vec<Existence>) {
        *self.existence.borrow_mut() = Some(state);
    }

    /// The existence state at the current point, taken: the block's exit
    /// once its statements have run. `None` when the run computes none.
    pub(crate) fn existence_leave(&self) -> Option<Vec<Existence>> {
        self.existence.borrow_mut().take()
    }

    /// The existence fact `symbol` holds at the current point, when the
    /// run computes existence.
    pub(crate) fn existence_now(&self, symbol: Symbol) -> Option<Existence> {
        self.existence
            .borrow()
            .as_ref()
            .and_then(|state| state.get(symbol.0 as usize).copied())
    }

    /// The existence state at the current point, copied, when the run
    /// computes existence.
    pub(crate) fn existence_state(&self) -> Option<Vec<Existence>> {
        self.existence.borrow().clone()
    }

    /// The existence state the scripts of the statement about to be
    /// evaluated start from ([`LatticeDriver::body_entry`]), held while the
    /// returned scope lives: dropping it clears the state, so no read after
    /// the statement answers from it.
    pub(crate) fn body_entry_scope(&self, state: Option<Vec<Existence>>) -> BodyEntryScope<'_> {
        *self.body_entry.borrow_mut() = state;
        BodyEntryScope {
            entry: &self.body_entry,
        }
    }

    /// Advance `symbol`'s fact at the current point by `step`, returning
    /// the fact afterwards.
    pub(crate) fn existence_step(&self, symbol: Symbol, step: ExistenceStep) -> Option<Existence> {
        let mut state = self.existence.borrow_mut();
        let fact = state.as_mut()?.get_mut(symbol.0 as usize)?;
        *fact = step.apply(*fact);
        Some(*fact)
    }

    /// Replace every fact at the current point by `clobber`'s answer for
    /// it: a barrier's, or a computed name's.
    pub(crate) fn existence_clobber(&self, clobber: impl Fn(Existence) -> Existence) {
        if let Some(state) = self.existence.borrow_mut().as_mut() {
            for fact in state.iter_mut() {
                *fact = clobber(*fact);
            }
        }
    }

    /// The existence fact the place `name` holds at the current point, as
    /// an input view: `Unavailable` when the run computes none, or the name
    /// is no symbol of the function. Where the statement's scripts start from
    /// another state ([`LatticeDriver::body_entry`]) the fact is that state's.
    fn existence_fact(&self, ssa: &SsaFunction, name: &str) -> FactView {
        ssa.var_symbol(name)
            .or_else(|| self.existence_places.borrow().get(name).copied())
            .and_then(|symbol| match self.body_entry.borrow().as_ref() {
                Some(state) => state.get(symbol.0 as usize).copied(),
                None => self.existence_now(symbol),
            })
            .map_or(
                FactView::Top(DeclineReason::Unavailable(self.context.tier)),
                |fact| FactView::Domain(DomainFact::Existence(fact)),
            )
    }

    /// Hand the driver the rung's slots past the SSA's symbols: each place
    /// only an existence query names, and the slot the run keeps it in.
    pub(crate) fn existence_places(&self, places: HashMap<String, Symbol>) {
        *self.existence_places.borrow_mut() = places;
    }

    /// Hand the driver, per slot, whether the rung holds the place
    /// externally mutable.
    pub(crate) fn existence_external(&self, external: Vec<bool>) {
        *self.existence_external.borrow_mut() = external;
    }

    /// Hand the driver the function's flattened `catch` regions.
    pub(crate) fn catch_ends(&self, cfg: &crate::cfg::Function) {
        *self.catch_ends.borrow_mut() = cfg
            .catch_ends
            .iter()
            .filter_map(|end| {
                let name = cfg.blocks.get(&end.end)?.name.clone();
                let record = CatchEndRecord {
                    entry: end.entry,
                    call: end.call.clone(),
                };
                Some((name, record))
            })
            .collect();
    }

    /// The result and options variables of a flattened `catch`, which the
    /// statement that ends its region defines. That statement stands where the
    /// body's flow has joined and has no words, so the `catch` the analysis
    /// build kept beside it is evaluated as the statement form is, over the
    /// state before the body — the versions the block before the body exits
    /// with, and the existence `exit_of` gives for that block: the body's own
    /// statements are in the flow, so what the script wrote is not applied
    /// again, and a script that reads what it writes finds it as it was.
    /// `None` for every other statement.
    pub(crate) fn evaluate_catch_end<S: std::hash::BuildHasher>(
        &self,
        (block, index): (&crate::ssa::SsaBlock, usize),
        values: &HashMap<ValueKey, LatticeValue, S>,
        ssa: &SsaFunction,
        exit_of: impl FnOnce(crate::cfg::BlockId) -> Option<Vec<Existence>>,
    ) -> Option<DefValues> {
        if index != 0 {
            return None;
        }
        let (entry, call) = {
            let ends = self.catch_ends.borrow();
            let end = ends.get(block.name.as_str())?;
            (end.entry, end.call.clone())
        };
        let marker = block.statements.first()?;
        let before = ssa.blocks.get(&entry)?.exit_versions.clone();
        let shadow = SsaStatement {
            statement: call,
            uses: before.clone(),
            defs: marker.defs.clone(),
            may_defs: marker.may_defs.clone(),
            quoted_uses: HashSet::new(),
            name_only_uses: HashSet::new(),
        };
        // The script runs over the existence the block before the body
        // leaves, as it does over that block's versions.
        let _entry = self.body_entry_scope(exit_of(entry));
        self.explaining(Some(shadow.statement.span()));
        let answer = self.evaluate_call(&shadow, values, ssa, &before);
        self.explaining(None);
        Some(answer)
    }

    /// Whether the place `name` is one the rung holds externally mutable: a
    /// call to a procedure the module cannot see may bind or unset it, so
    /// no fact about it is proven at a point.
    fn existence_is_external(&self, ssa: &SsaFunction, name: &str) -> bool {
        ssa.var_symbol(name)
            .or_else(|| self.existence_places.borrow().get(name).copied())
            .is_some_and(|symbol| {
                self.existence_external
                    .borrow()
                    .get(symbol.0 as usize)
                    .copied()
                    .unwrap_or(false)
            })
    }

    /// The existence query `kind` over its source words: a literal
    /// name reads its place, and an element name its array; a computed key
    /// leaves a bareword array fixed, so `Params($k)` asks about `Params` as
    /// a literal element does. Any other computed name decides nothing.
    fn existence_answer(
        &self,
        kind: crate::existence_query::ExistenceKind,
        (words, texts): (&[InvocationWord<'_>], &[&str]),
        (ssa, prior): (&SsaFunction, &[(PlaceRef, StoreOutcome)]),
    ) -> LiftedAnswer {
        match (words, texts) {
            ([_, InvocationWord::Literal(name)], _) => {
                let base = crate::sccp::place_base(name);
                self.existence_of(kind, base, base != *name, (ssa, prior))
            }
            ([_, InvocationWord::Dynamic], [_, text]) => {
                crate::existence_query::computed_element_base(text)
                    .map_or(LiftedAnswer::Declined(DeclineReason::NotExact), |base| {
                        self.existence_of(kind, base, true, (ssa, prior))
                    })
            }
            _ => LiftedAnswer::Declined(DeclineReason::NotExact),
        }
    }

    /// `info exists NAME` / `array exists NAME` over the existence rung at
    /// the current point, for the place `base` — an `element` query
    /// names its array: a bound place exists, one bound as an array is an
    /// array, one bound as a scalar is no array, and an unbound one is
    /// neither; an element exists only in an array, so an unbound array
    /// holds none, whatever the key. Anything else decides nothing —
    /// `MayBound`, `Bound(Either)` for `array exists`, an element of an
    /// array that may exist, a run with no rung (`Unavailable`, never read
    /// as unbound), a special variable in the initial global frame, which
    /// the host rather than the script binds, and any other externally
    /// mutable place, which a call the module cannot see may bind
    /// or unset.
    fn existence_of(
        &self,
        kind: crate::existence_query::ExistenceKind,
        base: &str,
        element: bool,
        (ssa, prior): (&SsaFunction, &[(PlaceRef, StoreOutcome)]),
    ) -> LiftedAnswer {
        use crate::existence_query::ExistenceKind;
        // An evaluation that has already written the place, or its array,
        // has moved it off the rung's point: the query decides nothing.
        if !matches!(written_in(prior, base), WrittenPlace::Untouched) {
            return LiftedAnswer::Declined(DeclineReason::StatefulNested);
        }
        let host_bound = self.existence_initial_global
            && self
                .registry
                .special_var_in_dialect(
                    base,
                    Some(tcl_registry::special_vars::surface_query_for_profile(
                        self.registry.profile(),
                    )),
                )
                .is_some();
        if host_bound || self.existence_is_external(ssa, base) {
            return LiftedAnswer::Declined(DeclineReason::EscapingPlace);
        }
        let fact = match self.existence_fact(ssa, base) {
            FactView::Domain(DomainFact::Existence(fact)) => fact,
            FactView::Top(reason) => return LiftedAnswer::Declined(reason),
            _ => return LiftedAnswer::Declined(DeclineReason::Unsupported),
        };
        let exists = match (kind, element, fact) {
            (_, _, Existence::Pending) => return LiftedAnswer::Pending,
            (_, _, Existence::Unbound)
            | (ExistenceKind::Array, false, Existence::Bound(BindingKind::Scalar)) => false,
            (ExistenceKind::AnyVariable, false, Existence::Bound(_))
            | (ExistenceKind::Array, false, Existence::Bound(BindingKind::Array)) => true,
            _ => return LiftedAnswer::Declined(DeclineReason::Unsupported),
        };
        LiftedAnswer::Evaluated(vec![Box::new(InvocationOutcome {
            completion: CompletionOutcome::Normal,
            nested_writes: Vec::new(),
            result: ExactValueOrUnavailable::Exact(ExactValue::int(i64::from(exists))),
            ordered_stores: Vec::new(),
            types: TypeFacts {
                result: Some(TclType::Boolean),
                per_target: Vec::new(),
                shapes: Vec::new(),
            },
            evidence: DependencyEvidence::default(),
        })])
    }

    /// The template-word plan each call of `block` declares, over the
    /// settled lattice — a switch's proven value reads as its spelling —
    /// with the template word's span. Only a trusted call to a command that
    /// performs substitution is asked.
    pub(crate) fn template_plans_in<S: std::hash::BuildHasher>(
        &self,
        ssa: &SsaFunction,
        block: &crate::ssa::SsaBlock,
        values: &HashMap<ValueKey, LatticeValue, S>,
    ) -> Vec<crate::sccp::TemplatePlanRecord> {
        let mut records = Vec::new();
        for stmt_ssa in &block.statements {
            let Statement::Call {
                command,
                args,
                tokens: Some(tokens),
                foreach_groups: None,
                ..
            } = &stmt_ssa.statement
            else {
                continue;
            };
            let head = stmt_ssa.statement.canonical_command_or_source();
            if tokens.argv.len() != args.len() + 1 || !self.trusted(head) {
                continue;
            }
            let cooked = call_arguments(args, Some(tokens), &self.lexer_config);
            let texts: Vec<&str> = cooked.iter().map(|arg| arg.text.as_ref()).collect();
            let words: Vec<InvocationWord<'_>> = cooked.iter().map(ArgWord::word).collect();
            let Some(resolved) = self.resolve(head, &words) else {
                continue;
            };
            if !resolved
                .semantics
                .traits
                .contains(tcl_registry::Traits::PERFORMS_SUBSTITUTION)
            {
                continue;
            }
            let Some(semantics) = resolved.semantics.value.semantics() else {
                continue;
            };
            let inputs = LatticeInputs {
                driver: self,
                prior_writes: Vec::new(),
                words: Words::Independent,
                view: view_of(&resolved, &texts, &words, InvocationLayout::Source),
                uses: &stmt_ssa.uses,
                values,
                ssa,
                sources: cooked.iter().map(|arg| arg.source).collect(),
            };
            if let PlanAnswer::TemplateWord(plan) = semantics.structure(&inputs)
                && let Some(&span) = tokens.argv.get(plan.operand.0 + 1)
            {
                // The spelling each switch reads as, when the lattice
                // proves every one exactly.
                let switches = (0..plan.operand.0)
                    .map(
                        |index| match inputs.operand(OperandId(index), FactDomain::ExactValue) {
                            FactView::Exact(value, _) => String::from_utf8(value.bytes).ok(),
                            _ => None,
                        },
                    )
                    .collect();
                records.push(crate::sccp::TemplatePlanRecord {
                    span,
                    command: command.clone(),
                    switches,
                    plan,
                });
            }
        }
        records
    }

    /// The selection each opaque case-list statement of `block` makes over
    /// the settled lattice ([`Self::selection_of`]), and the `Selected`
    /// branch fact of every arm that no member of a statement's subject
    /// reaches ([`unreached_arm_facts`]).
    pub(crate) fn selection_facts_in<S: std::hash::BuildHasher>(
        &self,
        cfg: &crate::cfg::Function,
        ssa: &SsaFunction,
        (block_id, block): (crate::cfg::BlockId, &crate::ssa::SsaBlock),
        values: &HashMap<ValueKey, LatticeValue, S>,
    ) -> (
        Vec<crate::sccp::SelectionRecord>,
        Vec<crate::sccp::ConstantBranch>,
    ) {
        let mut records = Vec::new();
        let mut unreached = Vec::new();
        for stmt_ssa in &block.statements {
            let Some(record) = self.selection_of(cfg, ssa, values, stmt_ssa) else {
                continue;
            };
            if let Statement::Switch { arms, .. } = &stmt_ssa.statement {
                unreached.extend(unreached_arm_facts(cfg.block_name(block_id), &record, arms));
            }
            records.push(record);
        }
        (records, unreached)
    }

    /// The selection one case-list statement makes over the settled
    /// lattice: the command its binding site names ([`binding_at`]) — trusted,
    /// and resolved over the statement's words — asked for its `Selection`
    /// transfer, the words read as the lowering recorded them
    /// ([`switch_arguments`]) and the variables at the statement's use
    /// versions. The fact's arm indices count the command's pattern and body
    /// pairs, which the record reads against the statement's arms and its
    /// final `default`, so a statement whose plan does not read the
    /// statement's own subject (an inliner's renamed one) or its own clause
    /// count, whose words' delimiters the lowering did not record, or whose
    /// transfer declines, has none.
    fn selection_of<S: std::hash::BuildHasher>(
        &self,
        cfg: &crate::cfg::Function,
        ssa: &SsaFunction,
        values: &HashMap<ValueKey, LatticeValue, S>,
        stmt_ssa: &SsaStatement,
    ) -> Option<crate::sccp::SelectionRecord> {
        let Statement::Switch {
            span,
            subject,
            arms,
            default_body,
            raw_args,
            raw_arg_braced,
            raw_arg_quoted,
            ..
        } = &stmt_ssa.statement
        else {
            return None;
        };
        if raw_arg_braced.len() != raw_args.len() || raw_arg_quoted.len() != raw_args.len() {
            return None;
        }
        let head = binding_at(cfg, *span)?;
        if !self.trusted(head) {
            return None;
        }
        let cooked = switch_arguments(
            raw_args,
            (raw_arg_braced, raw_arg_quoted),
            &self.lexer_config,
        );
        let texts: Vec<&str> = cooked.iter().map(|arg| arg.text.as_ref()).collect();
        let words: Vec<InvocationWord<'_>> = cooked.iter().map(ArgWord::word).collect();
        let resolved = self.resolve(head, &words)?;
        let semantics = resolved.semantics.value.semantics()?;
        let inputs = LatticeInputs {
            driver: self,
            prior_writes: Vec::new(),
            words: Words::Independent,
            view: view_of(&resolved, &texts, &words, InvocationLayout::Source),
            uses: &stmt_ssa.uses,
            values,
            ssa,
            sources: cooked.iter().map(|arg| arg.source).collect(),
        };
        let PlanAnswer::CaseList {
            subject: at,
            arms: plan_arms,
            ..
        } = semantics.structure(&inputs)
        else {
            return None;
        };
        let clauses = arms.len() + usize::from(default_body.is_some());
        if raw_args.get(at.0) != Some(subject)
            || plan_clause_count(
                &inputs,
                &plan_arms,
                WordValueRules::from_config(&self.lexer_config),
            ) != Some(clauses)
        {
            return None;
        }
        let TransferAnswer::Selection(fact) =
            semantics.transfer(FactDomain::Selection, &inputs, &mut self.budget())
        else {
            return None;
        };
        Some(crate::sccp::SelectionRecord {
            span: *span,
            arm_pattern_spans: arms.iter().map(|arm| arm.pattern_span).collect(),
            fact,
        })
    }

    /// What the run recorded beside the lattice, at its end: the route
    /// explanations and tally, the folded types and the preserved
    /// definitions, in an otherwise empty result.
    pub(crate) fn take_run_facts(&self) -> crate::sccp::SccpResult {
        crate::sccp::SccpResult {
            explanations: self.take_explanations(),
            route_tally: self.take_route_tally(),
            folded_types: self.take_folded_types(),
            preserved: self.take_preserved(),
            raised: self.take_raised(),
            reads_module: self.reads_module.get(),
            completion: crate::sccp::RunCompletion {
                raises: std::mem::take(&mut *self.uncaught.borrow_mut()),
                undecided: self.undecided.get(),
            },
            ..crate::sccp::SccpResult::default()
        }
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

    /// Resolve `head args…` through the invocation resolver under the
    /// registry's own surface.
    fn resolve<'w>(
        &self,
        head: &'w str,
        words: &'w [InvocationWord<'w>],
    ) -> Option<ResolvedInvocation<'a, 'w>>
    where
        'a: 'w,
    {
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
    ) -> DefValues {
        let widen = || DefValues::PerDef(widened(defs));
        let heads = typed_node_commands(self.registry, LoweringHookId::Incr);
        let Some(&head) = heads.first() else {
            return widen();
        };
        if !heads.iter().all(|head| self.trusted(head)) {
            self.explain(head, None, "declined: rebinding-suspected".to_owned());
            return widen();
        }
        let texts: Vec<&str> = std::iter::once(name)
            .chain(amount.map(|(text, _)| text))
            .collect();
        let words: Vec<InvocationWord<'_>> = std::iter::once(InvocationWord::Literal(name))
            .chain(amount.map(amount_word))
            .collect();
        let Some(resolved) = self.resolve(head, &words) else {
            return widen();
        };
        let Some(semantics) = resolved.semantics.value.semantics() else {
            return widen();
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
            prior_writes: Vec::new(),
            words: Words::Independent,
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
    ) -> DefValues {
        let widen = || DefValues::PerDef(self.transferred(semantics, defs, inputs, widened(defs)));
        let route = semantics.route();
        match route {
            EvalRoute::Direct { id } if id.owner() == EvaluatorOwner::Registry => {
                self.enter_direct();
            }
            // A declared implementation is registry-owned too: the
            // specialisation's `evaluate` runs it in the bounded host, on
            // the bindings it declares it rests on.
            EvalRoute::Implementation(capability) => {
                if self.explains_an_unheld_binding(head, route, &capability) {
                    return widen();
                }
                self.enter_implementation();
            }
            _ => {
                self.explain(
                    head,
                    Some(route),
                    "not evaluated: the route is not registry-owned".to_owned(),
                );
                return widen();
            }
        }
        self.word_error.set(false);
        let answer = evaluate_lifted(semantics, inputs, &mut self.budget(), MAX_CONSTSET_SIZE);
        self.explain(head, Some(route), answer_label(&answer));
        if let LiftedAnswer::Evaluated(outcomes) = &answer {
            self.explain_paths(outcome_paths(outcomes, Some(inputs)));
        }
        self.explain_transfer_paths(semantics, inputs);
        // A word that raises is an error before the command runs, whatever
        // the route would answer from the words it reads: nothing it would
        // store is stored, and the state before it is the one a handler is
        // thrown to.
        if self.word_error.replace(false) {
            if self.raises_end_paths() {
                return DefValues::Raised(Box::new(RaisedDefs {
                    prefix: None,
                    written: 0,
                }));
            }
            return widen();
        }
        let outcomes = match answer {
            LiftedAnswer::Pending => {
                let pending = defs
                    .iter()
                    .map(|(_, key)| DefAnswer {
                        existence: ExistenceStep::PENDING,
                        ..DefAnswer::untyped(*key, LatticeValue::Unknown)
                    })
                    .collect();
                return DefValues::PerDef(self.transferred(semantics, defs, inputs, pending));
            }
            LiftedAnswer::Declined(_) => return widen(),
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
            return widen();
        }
        // A completion that is not the normal one publishes no value for the
        // statement's definitions: where a raise ends the path, they take what
        // the stores that ran left, and elsewhere they widen. The explanation
        // already says what the route proved.
        if outcomes
            .iter()
            .any(|outcome| !outcome.completion.is_normal())
        {
            if self.raises_end_paths()
                && let Some(raised) = self.raised_defs(&outcomes, defs, inputs)
            {
                return DefValues::Raised(Box::new(raised));
            }
            return widen();
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
                    return widen();
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
        DefValues::PerDef(joined.unwrap_or_else(|| widened(defs)))
    }

    /// The statement's definitions when every outcome is an error: each
    /// after the stores that ran before it, the places the rest name left as
    /// they were, joined over the members of a finite input. `None` where an
    /// outcome is another completion, carries writes of its words, or
    /// stores to places the evaluation cannot place — the statement then
    /// widens as before.
    fn raised_defs(
        &self,
        outcomes: &[Box<InvocationOutcome>],
        defs: &[(String, ValueKey)],
        input: &dyn AnalysisInputs,
    ) -> Option<RaisedDefs> {
        let mut joined: Option<Vec<DefAnswer>> = None;
        let mut written = usize::MAX;
        for outcome in outcomes {
            let CompletionOutcome::Error { written: ran, .. } = outcome.completion else {
                return None;
            };
            if !outcome.nested_writes.is_empty() {
                return None;
            }
            let placed = self.placed_stores(outcome, input).ok()?;
            let ran_stores = placed.get(..ran)?;
            // A place the stores that ran did not reach keeps what it held,
            // an element of an untouched array included.
            let untouched = StoreOutcome::Preserve {
                target: TargetId(OperandId(0)),
            };
            let mut prefix: Vec<(PlaceRef, &StoreOutcome)> = ran_stores.to_vec();
            for (name, _) in defs {
                let place = place_named(name);
                if !ran_stores
                    .iter()
                    .any(|(seen, _)| seen.shares_storage_with(&place))
                {
                    prefix.push((place, &untouched));
                }
            }
            let answers = self.defs_from_placed(&prefix, Some(outcome), defs, input);
            written = written.min(ran);
            joined = Some(match joined {
                None => answers,
                Some(earlier) => earlier
                    .into_iter()
                    .zip(&answers)
                    .map(|(left, right)| left.join(right))
                    .collect(),
            });
        }
        Some(RaisedDefs {
            prefix: Some(joined?),
            written,
        })
    }

    /// Add to the statement's explanation the paths the declaration's
    /// existence transfer lists, when it lists an error path beside the
    /// normal one, for each kind of completion the evaluation did not
    /// already report: a kind the analysis cannot place the failing step of
    /// is still a path the statement may take.
    fn explain_transfer_paths(
        &self,
        semantics: &dyn CommandSemantics,
        inputs: &dyn AnalysisInputs,
    ) {
        let TransferAnswer::Existence(transfer) =
            semantics.transfer(FactDomain::Existence, inputs, &mut self.budget())
        else {
            return;
        };
        if transfer.paths.len() < 2 {
            return;
        }
        let Some(span) = self.explaining.get() else {
            return;
        };
        let mut explanations = self.explanations.borrow_mut();
        let Some(explanation) = explanations.get_mut(&(span.start(), span.end())) else {
            return;
        };
        for path in transfer_paths(&transfer, inputs) {
            let reported = explanation.paths.iter().any(|known| {
                known.completion == path.completion
                    || (path.completion == PathCompletion::Error(None)
                        && known.completion.is_error())
            });
            if !reported {
                explanation.paths.push(path);
            }
        }
    }

    /// `answers` with the existence each definition takes from the
    /// declaration's own existence transfer, when it states one: the
    /// normal completion's outcome for each target's place
    /// ([`existence_from`]). A definition the transfer does not name, and
    /// every definition of a command whose transfer is generic, keeps the
    /// step its answer carries — a pending evaluation's, or the generic
    /// widening.
    fn transferred(
        &self,
        semantics: &dyn CommandSemantics,
        defs: &[(String, ValueKey)],
        inputs: &dyn AnalysisInputs,
        mut answers: Vec<DefAnswer>,
    ) -> Vec<DefAnswer> {
        let TransferAnswer::Existence(transfer) =
            semantics.transfer(FactDomain::Existence, inputs, &mut self.budget())
        else {
            return answers;
        };
        let Some(normal) = transfer
            .paths
            .iter()
            .find(|path| completes_normally(path.completion))
        else {
            return answers;
        };
        let steps: Vec<(PlaceRef, ExistenceStep)> = normal
            .outcomes
            .iter()
            .filter_map(|(target, outcome)| {
                inputs
                    .place(target.0)
                    .ok()
                    .map(|place| (place, ExistenceStep::of(*outcome)))
            })
            .collect();
        for (answer, (name, _)) in answers.iter_mut().zip(defs) {
            if let Some(step) = existence_from(&steps, name) {
                answer.existence = step;
            }
        }
        answers
    }

    /// The value one outcome leaves in each of `defs`, the statement's
    /// definitions (`docs/design/compiler/value-transfers.md` § *Storage-
    /// writing commands*). Every store's target resolves to its place
    /// first — an element write's to the element of its target's array —
    /// so two spellings of one cell are one place and a repeated target
    /// composes in execution order: a `Write` is its value, the last write
    /// winning; a `Preserve` keeps what the place holds at that
    /// point, the prior version's value when nothing earlier wrote it (a
    /// pending prior stays pending); a `MayWrite` and an `Unbind` widen,
    /// their facts being the type domain's and the existence rung's. A
    /// definition no store names widens.
    ///
    /// # Errors
    ///
    /// The whole answer declines, and every definition widens, for a
    /// completion other than the normal one (the stores made before an error
    /// are not applied); a target that is no place (`DynamicName`, `EscapingPlace`
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
        let placed = self.placed_stores(outcome, input)?;
        Ok(self.defs_from_placed(&placed, Some(outcome), defs, input))
    }

    /// The value each of `defs` takes from `placed`, the stores an outcome
    /// makes in execution order with the place each names: the last write to
    /// a place wins, a preserve keeps what the place holds at that point, a
    /// may-write or an unbind widens, and a definition no store names
    /// widens. `outcome` supplies the folded type its stores state, when the
    /// stores are its own.
    fn defs_from_placed(
        &self,
        placed: &[(PlaceRef, &StoreOutcome)],
        outcome: Option<&InvocationOutcome>,
        defs: &[(String, ValueKey)],
        input: &dyn AnalysisInputs,
    ) -> Vec<DefAnswer> {
        let steps: Vec<(PlaceRef, ExistenceStep)> = placed
            .iter()
            .map(|(place, store)| (place.clone(), store_existence(store)))
            .collect();
        defs.iter()
            .map(|(name, key)| {
                let existence = existence_from(&steps, name).unwrap_or(ExistenceStep::UNKNOWN);
                let named: Vec<&(PlaceRef, &StoreOutcome)> = placed
                    .iter()
                    .filter(|(place, _)| place.name == *name)
                    .collect();
                let Some((place, _)) = named.first() else {
                    return DefAnswer {
                        existence,
                        ..DefAnswer::untyped(*key, LatticeValue::Overdefined)
                    };
                };
                if self.is_escaping(place) {
                    return DefAnswer {
                        existence,
                        ..DefAnswer::untyped(*key, LatticeValue::Overdefined)
                    };
                }
                let mut held: Option<LatticeValue> = None;
                for (_, store) in &named {
                    match store {
                        StoreOutcome::Write { value, .. }
                        | StoreOutcome::WriteElement { value, .. } => {
                            held = Some(exact_to_lattice(value));
                        }
                        StoreOutcome::Preserve { .. } => {}
                        StoreOutcome::Unbind { .. }
                        | StoreOutcome::MayWrite { .. }
                        | StoreOutcome::WriteUnavailable { .. } => {
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
                    folded: outcome.and_then(|outcome| {
                        FoldedType::after_stores(outcome, named.iter().map(|(_, store)| *store))
                    }),
                    stated: true,
                    preserved: named
                        .iter()
                        .all(|(_, store)| matches!(store, StoreOutcome::Preserve { .. })),
                    existence,
                }
            })
            .collect()
    }

    /// Each of an outcome's ordered stores with the place it names, in
    /// execution order: an element write's target resolves to the element
    /// of its array, so two spellings of one cell are one place.
    ///
    /// # Errors
    ///
    /// A target that is no place (`DynamicName`, `EscapingPlace` from the
    /// resolver); an element write beside its array's base write
    /// (`OverlappingTargets`); and a traced place (`TracedPlace`), whose
    /// trace runs on the write and can observe or rewrite the others.
    fn placed_stores<'o>(
        &self,
        outcome: &'o InvocationOutcome,
        input: &dyn AnalysisInputs,
    ) -> Result<Vec<(PlaceRef, &'o StoreOutcome)>, DeclineReason> {
        let mut placed: Vec<(PlaceRef, &StoreOutcome)> =
            Vec::with_capacity(outcome.ordered_stores.len());
        for store in &outcome.ordered_stores {
            let place = match store {
                StoreOutcome::WriteElement { target, key, .. } => {
                    element_place(&input.place(target.0)?, key)?
                }
                _ => input.place(store.target().0)?,
            };
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
        Ok(placed)
    }

    /// Whether the ordered evaluation state can own a write to `place`:
    /// local to the frame, so no other frame or trace can write or observe
    /// it while the evaluation runs — not traced, not escaping, and not an
    /// element of an array that is.
    fn state_owns(&self, place: &PlaceRef) -> bool {
        !self.is_traced(place)
            && !self.is_escaping(place)
            && !self.is_escaping(&PlaceRef::scalar(place.base()))
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

    /// A statement's word effects and the statement whose words they are,
    /// paired by construction ([`crate::ssa::word_effects_host`]) and
    /// evaluated once (§ `expr`): the host's words run in order under one
    /// ordered state, from the versions the definition point reads. Its
    /// definitions take what the state's writes left in each place — a place
    /// no write reached keeps what it held — and the host's take the
    /// result. `None` wherever the pair is not one this evaluates or the
    /// evaluation declines, and both keep the answers they have without it.
    ///
    /// The host is an `AssignExpr` or an `ExprEval`, whose expression the
    /// engine runs, or an `AssignValue` whose word is one command
    /// substitution on the engine's or a registry-owned route, whose own
    /// substituting words and nested commands are the ones evaluated in
    /// order. Every other host — a `Call`, a `Return`, a condition, a value
    /// that is a command without such a route — keeps the effect-free
    /// policy.
    pub(crate) fn evaluate_embedded<S: std::hash::BuildHasher>(
        &self,
        block: &crate::ssa::SsaBlock,
        index: usize,
        values: &HashMap<ValueKey, LatticeValue, S>,
        ssa: &SsaFunction,
    ) -> Option<(usize, EmbeddedAnswer)> {
        let host_index = crate::ssa::word_effects_host(&block.statements, index)?;
        let (call, host) = (&block.statements[index], &block.statements[host_index]);
        let incoming = incoming_versions(block, (index, host_index), values);
        let started = self.tally.get();
        self.explaining(Some(call.statement.span()));
        let answer = self.embedded_pair(call, host, (&incoming, values, ssa));
        self.explaining(None);
        if answer.is_none() {
            self.tally.set(started);
        }
        answer.map(|answer| (host_index, answer))
    }

    /// [`Self::evaluate_embedded`] over the versions in force where the
    /// call sits.
    fn embedded_pair<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &self,
        call: &SsaStatement,
        host: &SsaStatement,
        lattice: Lattice<'_, S1, S2>,
    ) -> Option<EmbeddedAnswer> {
        let ordered = match &host.statement {
            Statement::AssignExpr {
                expr,
                command_binding,
                ..
            } => self.ordered_expression(expr, command_binding.as_ref(), lattice)?,
            Statement::ExprEval {
                expr,
                command_binding,
                ..
            } => self.ordered_expression(expr, Some(command_binding), lattice)?,
            Statement::AssignValue { value, .. } => self.ordered_script(value, lattice)?,
            _ => return None,
        };
        let defs = named_defs(call, lattice.2);
        let outcomes = match ordered.answer {
            LiftedAnswer::Pending => {
                let pending = defs
                    .iter()
                    .map(|(_, key)| DefAnswer {
                        existence: ExistenceStep::PENDING,
                        ..DefAnswer::untyped(*key, LatticeValue::Unknown)
                    })
                    .collect();
                return Some(EmbeddedAnswer {
                    call: pending,
                    host: DefValues::Each(LatticeValue::Unknown, None),
                    raised: None,
                });
            }
            LiftedAnswer::Declined(_) => return None,
            LiftedAnswer::Evaluated(outcomes) => outcomes,
        };
        // An error completion publishes only the writes it ran (the prefix
        // rule): where a raise ends the path the call's definitions take them
        // and the host's keep what they held, and elsewhere the pair is not
        // evaluated.
        let mut raised: Option<usize> = None;
        let errors = outcomes
            .iter()
            .filter(|outcome| matches!(outcome.completion, CompletionOutcome::Error { .. }))
            .count();
        if errors > 0 {
            if errors < outcomes.len() || !self.raises_end_paths() {
                return None;
            }
            raised = outcomes
                .iter()
                .filter_map(|outcome| match outcome.completion {
                    CompletionOutcome::Error { written, .. } => Some(written),
                    _ => None,
                })
                .min();
        } else if outcomes
            .iter()
            .any(|outcome| outcome.completion != CompletionOutcome::Normal)
        {
            return None;
        }
        let inputs = self.expression_inputs(lattice);
        let mut joined: Option<Vec<DefAnswer>> = None;
        for member in &ordered.writes {
            let answers = self.embedded_defs(member, &defs, &inputs)?;
            joined = Some(match joined {
                None => answers,
                Some(earlier) => earlier
                    .into_iter()
                    .zip(&answers)
                    .map(|(left, right)| left.join(right))
                    .collect(),
            });
        }
        if let Some(written) = raised {
            return Some(EmbeddedAnswer {
                call: joined?,
                host: DefValues::Raised(Box::new(RaisedDefs {
                    prefix: None,
                    written,
                })),
                raised,
            });
        }
        let value = lattice_of_outcomes(&outcomes, result_of);
        let folded = ordered
            .folded
            .filter(|_| value != LatticeValue::Overdefined);
        Some(EmbeddedAnswer {
            call: joined?,
            host: DefValues::Each(value, folded),
            raised: None,
        })
    }

    /// The parsed expression a host runs, evaluated by the engine under
    /// `LocalWrites` from the versions the call reads. `None` when the
    /// module rebinds the head.
    fn ordered_expression<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &self,
        expr: &ExprNode,
        command_binding: Option<&tcl_runtime_api::CommandBindingIdentity>,
        (uses, values, ssa): Lattice<'_, S1, S2>,
    ) -> Option<Ordered> {
        let head = command_binding.map_or("expr", |binding| binding.name.as_str());
        if self.folds.is_some() && !self.trusted(head) {
            return None;
        }
        self.enter_expression();
        let expression = ExpressionEvaluation {
            expression: Expression::Parsed(expr),
            policy: self.policy,
            nested: NestedPolicy::LocalWrites,
            head: Some(binding_of(
                head,
                command_binding.map_or("expr", |binding| binding.identity.as_str()),
            )),
        };
        let answer = self.evaluate_expression_at(&expression, uses, values, ssa);
        self.explain(head, Some(expression.route()), answer_label(&answer));
        if let LiftedAnswer::Evaluated(outcomes) = &answer {
            self.explain_paths(outcome_paths(outcomes, None));
        }
        let writes = match &answer {
            LiftedAnswer::Evaluated(outcomes) => outcomes
                .iter()
                .map(|outcome| outcome.nested_writes.clone())
                .collect(),
            LiftedAnswer::Pending | LiftedAnswer::Declined(_) => Vec::new(),
        };
        Some(Ordered {
            answer,
            writes,
            folded: None,
        })
    }

    /// A value word that is one command substitution, run under
    /// `LocalWrites`: the invocation's own substituting words and the
    /// commands nested in them evaluate in order, its own stores follow, and
    /// the result is the word's. A command whose route is neither the
    /// expression engine's nor a registry-owned evaluator, and a word that
    /// is more than the one substitution, stay with the effect-free policy.
    fn ordered_script<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &self,
        value: &str,
        lattice: Lattice<'_, S1, S2>,
    ) -> Option<Ordered> {
        use tcl_lexer::word_parts::{SubstFlags, WordBody, WordPart as Part, decompose};
        // The word is that one substitution and nothing else: `[a][b]` is two.
        let WordBody::Parts(parts) =
            decompose(value.as_bytes(), SubstFlags::default(), self.lexer_config)
        else {
            return None;
        };
        let [Part::Command(script)] = parts.as_slice() else {
            return None;
        };
        let inner = std::str::from_utf8(script).ok()?;
        let run = self.run_script(inner, lattice, (Vec::new(), NestedPolicy::LocalWrites))?;
        // A head the module rebinds has no route; a procedure of the module
        // is run as its summary says.
        if !run.procedure
            && !matches!(
                run.route,
                Some(EvalRoute::Expression { .. } | EvalRoute::Direct { .. })
            )
        {
            return None;
        }
        self.explain(&run.head, run.route, answer_label(&run.answer));
        let folded = match &run.answer {
            LiftedAnswer::Evaluated(outcomes) => FoldedType::join_all(
                outcomes
                    .iter()
                    .map(|outcome| FoldedType::of_result(outcome)),
            ),
            LiftedAnswer::Pending | LiftedAnswer::Declined(_) => None,
        };
        Some(Ordered {
            answer: run.answer,
            writes: run.writes,
            folded,
        })
    }

    /// The call's definitions after one member's `writes`: a place the
    /// writes name takes the last value written, and one they leave alone
    /// keeps what it held. `None` when a write names a place the call does
    /// not define — no definition would carry it — or two writes name one
    /// cell as an element and its array.
    fn embedded_defs(
        &self,
        writes: &[(PlaceRef, StoreOutcome)],
        defs: &[(String, ValueKey)],
        input: &dyn AnalysisInputs,
    ) -> Option<Vec<DefAnswer>> {
        if writes
            .iter()
            .any(|(place, _)| !defs.iter().any(|(name, _)| *name == place.name))
        {
            return None;
        }
        let mut placed: Vec<(PlaceRef, &StoreOutcome)> = Vec::with_capacity(writes.len());
        for (place, store) in writes {
            if placed
                .iter()
                .any(|(seen, _)| seen.overlaps_as_element_and_base(place))
            {
                return None;
            }
            placed.push((place.clone(), store));
        }
        let untouched = StoreOutcome::Preserve {
            target: TargetId(OperandId(0)),
        };
        for (name, _) in defs {
            let place = place_named(name);
            if !placed
                .iter()
                .any(|(seen, _)| seen.shares_storage_with(&place))
            {
                placed.push((place, &untouched));
            }
        }
        Some(self.defs_from_placed(&placed, None, defs, input))
    }

    /// The inputs of an expression that has no operand words of its own:
    /// it reads every variable by name, at the versions `lattice` selects.
    fn expression_inputs<'i, S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &'i self,
        (uses, values, ssa): Lattice<'i, S1, S2>,
    ) -> LatticeInputs<'i, S1, S2> {
        LatticeInputs {
            driver: self,
            prior_writes: Vec::new(),
            words: Words::Independent,
            view: ResolvedInvocationView {
                canonical_command: "expr",
                subcommand: None,
                form: None,
                layout: InvocationLayout::Source,
                operands: Vec::new(),
                argument_offset: 0,
                arity: None,
            },
            uses,
            values,
            ssa,
            sources: Vec::new(),
        }
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
    ) -> DefValues {
        let defs = named_defs(stmt_ssa, ssa);
        let Statement::Call {
            args,
            defs: binders,
            tokens,
            foreach_groups,
            ..
        } = &stmt_ssa.statement
        else {
            return DefValues::PerDef(widened(&defs));
        };
        let head = stmt_ssa.statement.canonical_command_or_source();
        if !self.trusted(head) {
            self.explain(head, None, "declined: rebinding-suspected".to_owned());
            return DefValues::PerDef(widened(&defs));
        }
        if foreach_groups.is_none() {
            let cooked = call_arguments(args, tokens.as_ref(), &self.lexer_config);
            return self.evaluate_source_call(head, &cooked, &defs, (uses, values, ssa), true);
        }
        let (bound, existence) = self.evaluate_loop_header(head, args, binders, uses, values, ssa);
        DefValues::PerDef(
            defs.iter()
                .map(|(name, key)| {
                    let value = bound
                        .iter()
                        .find(|(binder, _)| binder == name)
                        .map_or(LatticeValue::Overdefined, |(_, value)| value.clone());
                    DefAnswer {
                        existence,
                        ..DefAnswer::untyped(*key, value)
                    }
                })
                .collect(),
        )
    }

    /// The existence step the registry's write class gives a call's
    /// definition `name` where the step its declared transfer took,
    /// `declared`, states less: a may-bind, or the generic widening of a
    /// command that declares no existence transfer or whose transfer
    /// declined. A target a command writes whenever it completes
    /// (`UNCONDITIONAL_VARIABLE_WRITE`: `gets`, `lassign`, `file tempfile`) is
    /// bound afterwards, as the kind a may-write declaration names or a
    /// scalar; one it writes only on a match or reads before writing
    /// (`CONDITIONAL_VARIABLE_WRITE`, `READS_BEFORE_WRITE`: `regexp`, `lset`)
    /// holds what the place held joined with a scalar binding. A target is a
    /// word at one of the invocation's own variable-write positions; an
    /// alias's prepended words are not on the call, so through one any word
    /// is, unless a word runs a substitution that could define a name of its
    /// own. `None` keeps `declared`: a head the module may rebind, a command
    /// that may destroy a variable (`unset`'s class, or an irreversible
    /// descriptor such as `array unset`'s), any other declared step, and any
    /// definition no write class reaches.
    pub(crate) fn existence_by_write_class(
        &self,
        statement: &Statement,
        name: &str,
        declared: ExistenceStep,
    ) -> Option<ExistenceStep> {
        let Statement::Call {
            canonical_command,
            args,
            tokens,
            foreach_groups: None,
            ..
        } = statement
        else {
            return None;
        };
        let may_bind = match declared {
            ExistenceStep::Join(Existence::Bound(kind)) => Some(kind),
            step if step == ExistenceStep::UNKNOWN => None,
            _ => return None,
        };
        let head = statement.canonical_command_or_source();
        if !self.trusted(head) {
            return None;
        }
        let cooked = call_arguments(args, tokens.as_ref(), &self.lexer_config);
        let texts: Vec<&str> = cooked.iter().map(|arg| arg.text.as_ref()).collect();
        let surface = self.registry.own_surface_query();
        let traits = self.registry.invocation_traits(head, &texts, surface);
        if traits.contains(tcl_registry::Traits::DESTROYS_VARIABLE)
            || self
                .registry
                .resolve_call(head, &texts, surface)
                .is_some_and(|call| call.sub.is_some_and(|sub| sub.destructive))
        {
            return None;
        }
        let step = if traits.contains(tcl_registry::Traits::UNCONDITIONAL_VARIABLE_WRITE) {
            ExistenceStep::Set(Existence::Bound(may_bind.unwrap_or(BindingKind::Scalar)))
        } else if may_bind.is_none()
            && traits.intersects(
                tcl_registry::Traits::CONDITIONAL_VARIABLE_WRITE
                    .union(tcl_registry::Traits::READS_BEFORE_WRITE),
            )
        {
            ExistenceStep::Join(Existence::Bound(BindingKind::Scalar))
        } else {
            return None;
        };
        let target = if canonical_command.is_some() {
            !args.iter().any(|word| word.contains('[')) && texts.contains(&name)
        } else {
            self.registry
                .arg_indices_for_role(head, &texts, ArgRole::VarWrite)
                .into_iter()
                .any(|index| texts.get(index).is_some_and(|&text| text == name))
        };
        target.then_some(step)
    }

    /// Whether a statement with no definitions certainly raises, where a
    /// throw leaves from: the answer is `Some` only for a statement the
    /// registry's routes prove an error. A call to a command with no
    /// semantics records nothing, as the solver evaluates no statement
    /// that has no definition anywhere else.
    pub(crate) fn probe_completion<S: std::hash::BuildHasher>(
        &self,
        stmt_ssa: &SsaStatement,
        values: &HashMap<ValueKey, LatticeValue, S>,
        ssa: &SsaFunction,
    ) -> Option<DefValues> {
        self.explaining(Some(stmt_ssa.statement.span()));
        let answer = match &stmt_ssa.statement {
            Statement::Call {
                args,
                tokens,
                foreach_groups: None,
                ..
            } => {
                let head = stmt_ssa.statement.canonical_command_or_source();
                self.trusted(head).then(|| {
                    let cooked = call_arguments(args, tokens.as_ref(), &self.lexer_config);
                    self.evaluate_source_call(
                        head,
                        &cooked,
                        &[],
                        (&stmt_ssa.uses, values, ssa),
                        false,
                    )
                })
            }
            Statement::ExprEval {
                expr,
                command_binding,
                ..
            } => Some(self.evaluate_expr_eval(expr, command_binding, &stmt_ssa.uses, values, ssa)),
            _ => None,
        };
        self.explaining(None);
        answer.filter(|answer| matches!(answer, DefValues::Raised(_)))
    }

    /// An `expr` statement's expression run by the engine, which has no
    /// definition to give: it raises or it does not.
    pub(crate) fn evaluate_expr_eval<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &self,
        expr: &ExprNode,
        command_binding: &tcl_runtime_api::CommandBindingIdentity,
        uses: &HashMap<Symbol, Version, S1>,
        values: &HashMap<ValueKey, LatticeValue, S2>,
        ssa: &SsaFunction,
    ) -> DefValues {
        let normal = || DefValues::Each(LatticeValue::Overdefined, None);
        if self.folds.is_some() && !self.trusted(&command_binding.name) {
            return normal();
        }
        let Some(ordered) =
            self.ordered_expression(expr, Some(command_binding), (uses, values, ssa))
        else {
            return normal();
        };
        match Self::raised_expression(&ordered.answer) {
            Some(raised) => DefValues::Raised(Box::new(raised)),
            None => normal(),
        }
    }

    /// The error an expression's answer certainly ends in, for a statement
    /// that stores nothing of its own: the writes the expression's commands
    /// made before it are the call's definitions, never this statement's.
    fn raised_expression(answer: &LiftedAnswer) -> Option<RaisedDefs> {
        let LiftedAnswer::Evaluated(outcomes) = answer else {
            return None;
        };
        let mut written = usize::MAX;
        for outcome in outcomes {
            let CompletionOutcome::Error { written: ran, .. } = outcome.completion else {
                return None;
            };
            written = written.min(ran);
        }
        (!outcomes.is_empty()).then_some(RaisedDefs {
            prefix: None,
            written,
        })
    }

    /// The synthetic loop header's value for each binder of its iteration
    /// plan over one list: the set of the elements it takes — binder `i` of
    /// `n` the elements at `i`, `i + n`, …, and the empty string where the
    /// last iteration runs past the list's end. A repeated binder takes its
    /// last position's. A multi-list header, an empty list, or a list the
    /// analysis cannot read leaves every binder `Overdefined`.
    ///
    /// Beside the values, the existence step every binder takes at the
    /// header: bound once the list is proven to have an element — every
    /// later visit of the header follows an iteration that bound it —
    /// left as it was when the list is proven empty and the plan binds
    /// nothing on zero iterations, and otherwise a may-bind.
    fn evaluate_loop_header<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &self,
        head: &str,
        args: &[String],
        defs: &[String],
        uses: &HashMap<Symbol, Version, S1>,
        values: &HashMap<ValueKey, LatticeValue, S2>,
        ssa: &SsaFunction,
    ) -> (Vec<(String, LatticeValue)>, ExistenceStep) {
        const MAY_BIND: ExistenceStep = ExistenceStep::Join(Existence::Bound(BindingKind::Scalar));
        const BINDS: ExistenceStep = ExistenceStep::Set(Existence::Bound(BindingKind::Scalar));
        let texts: Vec<&str> = args.iter().map(String::as_str).collect();
        let words: Vec<InvocationWord<'_>> = texts.iter().copied().map(word_of).collect();
        let Some(resolved) = self.resolve(head, &words) else {
            return (Vec::new(), MAY_BIND);
        };
        let Some(semantics) = resolved.semantics.value.semantics() else {
            self.explain(head, None, "declined: no-semantics".to_owned());
            return (Vec::new(), MAY_BIND);
        };
        let view = view_of(
            &resolved,
            &texts,
            &words,
            InvocationLayout::LoopHeader { binders: defs },
        );
        let inputs = LatticeInputs {
            driver: self,
            prior_writes: Vec::new(),
            words: Words::Independent,
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
            return (Vec::new(), MAY_BIND);
        };
        let IterableKind::List(iterable) = &plan.iterable else {
            return (Vec::new(), MAY_BIND);
        };
        let Some(list) = texts.get(iterable.0) else {
            return (Vec::new(), MAY_BIND);
        };
        let binders: Vec<&str> = plan
            .binders
            .iter()
            .filter_map(|binder| match &binder.name {
                BinderName::Declared(name) => Some(name.as_str()),
                BinderName::Operand(id) => texts.get(id.0).copied(),
            })
            .collect();
        if binders.len() != plan.binders.len() || binders.is_empty() {
            return (Vec::new(), MAY_BIND);
        }
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
        let existence = match &elements {
            Some(items) if !items.is_empty() || plan.zero_iterations_bind => BINDS,
            Some(_) => ExistenceStep::PRESERVE,
            None => MAY_BIND,
        };
        let Some(items) = elements.filter(|items| !items.is_empty()) else {
            return (Vec::new(), existence);
        };
        let stride = binders.len();
        let bound = binders
            .iter()
            .enumerate()
            .map(|(at, name)| {
                // A repeated binder is assigned at each of its positions, so
                // it holds its last one's element.
                let position = binders
                    .iter()
                    .rposition(|other| other == name)
                    .unwrap_or(at);
                let consts: Vec<ConstValue> = (0..items.len().div_ceil(stride))
                    .map(|iteration| {
                        let element = items
                            .get(iteration * stride + position)
                            .map_or("", String::as_str);
                        exact_to_const(&ExactValue::from_literal(element))
                    })
                    .collect();
                let value = match consts.as_slice() {
                    [only] => LatticeValue::Const(only.clone()),
                    _ => LatticeValue::constset(consts),
                };
                ((*name).to_owned(), value)
            })
            .collect();
        (bound, existence)
    }

    /// A call in its source layout, its arguments read as source words: the
    /// stores its registry-owned evaluation makes, applied to the variables
    /// it defines ([`Self::call_defs`]).
    fn evaluate_source_call<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &self,
        head: &str,
        cooked: &[ArgWord<'_>],
        defs: &[(String, ValueKey)],
        (uses, values, ssa): Lattice<'_, S1, S2>,
        explain_missing: bool,
    ) -> DefValues {
        let texts: Vec<&str> = cooked.iter().map(|arg| arg.text.as_ref()).collect();
        let words: Vec<InvocationWord<'_>> = cooked.iter().map(ArgWord::word).collect();
        let Some(resolved) = self.resolve(head, &words) else {
            return DefValues::PerDef(widened(defs));
        };
        let Some(semantics) = resolved.semantics.value.semantics() else {
            if explain_missing {
                self.explain(head, None, "declined: no-semantics".to_owned());
            }
            return DefValues::PerDef(widened(defs));
        };
        let view = view_of(&resolved, &texts, &words, InvocationLayout::Source);
        let mut inputs = LatticeInputs {
            driver: self,
            prior_writes: Vec::new(),
            words: Words::Independent,
            view,
            uses,
            values,
            ssa,
            sources: cooked.iter().map(|arg| arg.source).collect(),
        };
        inputs.resolve_roles_over_values(&resolved);
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
        let run = self.run_script(
            inner,
            (uses, values, ssa),
            (Vec::new(), NestedPolicy::EffectFreeOnly),
        )?;
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
                (LiftedAnswer::Evaluated(outcomes), None)
                    if outcomes.iter().any(|outcome| outcome.has_stores()) =>
                {
                    "not substituted: the outcome writes storage".to_owned()
                }
                _ => answer_label(&run.answer),
            },
        );
        folded
    }

    /// The one command a `[…]` script holds, resolved and run on its
    /// declared route over this statement's lattice inputs: a
    /// registry-owned evaluator or the expression engine. `None` when the
    /// script is not one command the registry resolves to a declaration.
    ///
    /// `prior` holds the writes the enclosing evaluations made before the
    /// command runs, which its reads consult first, and `policy` says
    /// whether the writes it makes are placed ([`ScriptRun::writes`]) for
    /// the caller to apply.
    fn run_script<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &self,
        script: &str,
        lattice: Lattice<'_, S1, S2>,
        under: (Vec<(PlaceRef, StoreOutcome)>, NestedPolicy),
    ) -> Option<ScriptRun> {
        let commands =
            crate::segmenter::segment_commands_with_offset_and_config(script, 0, self.lexer_config);
        let [seg] = commands.as_slice() else {
            return None;
        };
        if split_head(script).0 != seg.name() {
            return None;
        }
        self.run_command(seg, lattice, under)
    }

    /// A segmented command's arguments as the resolver and an evaluator read
    /// them, each cooked from its token. The segmenter drops a `{*}` prefix
    /// from the token and flags the word instead, so the flag decides that a
    /// word expands: `[list {*}{a b}]` has two elements, not the one `a b`.
    fn cooked_args<'s>(&self, seg: &'s crate::segmenter::SegmentedCommand) -> Vec<ArgWord<'s>> {
        let expands = |at: usize| {
            seg.expand_word
                .as_ref()
                .and_then(|flags| flags.get(at + 1))
                .copied()
                .unwrap_or(false)
        };
        seg.arg_tokens()
            .iter()
            .zip(seg.arg_single_token())
            .zip(seg.args())
            .enumerate()
            .map(|(at, ((token, &single), text))| {
                if expands(at) {
                    return ArgWord::spelled(
                        text,
                        InvocationWordKind::Expanded,
                        OperandSource::Unknown,
                    );
                }
                // A quoted-opening token counts its `"` as a delimiter byte.
                let quoted = token.kind == TokenType::Esc && token.content_offset > 0;
                ArgWord::of_token(text, (token.kind, quoted), single, &self.lexer_config)
            })
            .collect()
    }

    /// One segmented command resolved against the registry and evaluated on
    /// its declared route, `prior` being the writes made ahead of it.
    fn run_command<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &self,
        seg: &crate::segmenter::SegmentedCommand,
        (uses, values, ssa): Lattice<'_, S1, S2>,
        (prior, policy): (Vec<(PlaceRef, StoreOutcome)>, NestedPolicy),
    ) -> Option<ScriptRun> {
        if let Some(run) = self.procedure_run(seg, (uses, values, ssa), (&prior, policy)) {
            return Some(run);
        }
        let head = seg.name();
        if !self.trusted(head) {
            return Some(ScriptRun {
                head: head.to_owned(),
                route: None,
                answer: LiftedAnswer::Declined(DeclineReason::RebindingSuspected),
                writes: Vec::new(),
                binding: binding_of(head, head),
                rebound: true,
                procedure: false,
            });
        }
        let cooked = self.cooked_args(seg);
        let texts: Vec<&str> = cooked.iter().map(|arg| arg.text.as_ref()).collect();
        let words: Vec<InvocationWord<'_>> = cooked.iter().map(ArgWord::word).collect();
        let resolved = self.resolve(head, &words)?;
        let binding = binding_of(head, resolved.canonical_command);
        // An existence query reads the rung, not a route.
        if let Some(kind) = crate::existence_query::kind_of(resolved.semantics.operation) {
            return Some(ScriptRun {
                head: head.to_owned(),
                route: None,
                answer: self.existence_answer(kind, (&words, &texts), (ssa, &prior)),
                writes: Vec::new(),
                binding,
                rebound: false,
                procedure: false,
            });
        }
        let semantics = resolved.semantics.value.semantics()?;
        let view = view_of(&resolved, &texts, &words, InvocationLayout::Source);
        let mut inputs = LatticeInputs {
            driver: self,
            prior_writes: prior,
            words: Words::under(policy),
            view,
            uses,
            values,
            ssa,
            sources: cooked.iter().map(|arg| arg.source).collect(),
        };
        // A command that stores reads its targets' roles from its words as
        // a statement's does, so a word the lattice proves no longer hides
        // them (`set b $a`).
        if policy != NestedPolicy::EffectFreeOnly {
            inputs.resolve_roles_over_values(&resolved);
        }
        let route = semantics.route();
        // A word that raises ends the invocation before the command runs,
        // whichever operands the route reads; the flag stays set for the
        // word, statement or script that holds this command.
        let outer = self.word_error.replace(false);
        let answer = self.route_answer(semantics, &inputs, (&binding, policy));
        let raised = self.word_error.get();
        self.word_error.set(outer || raised);
        let answer = if raised {
            LiftedAnswer::Declined(DeclineReason::StatefulNested)
        } else {
            answer
        };
        let answer = inputs.carrying_word_writes(answer, route);
        let (answer, writes) = match policy {
            NestedPolicy::EffectFreeOnly => (answer, Vec::new()),
            NestedPolicy::LocalWrites | NestedPolicy::Protected => {
                self.placed_answer(semantics, &inputs, answer)
            }
        };
        Some(ScriptRun {
            head: head.to_owned(),
            route: Some(route),
            answer,
            writes,
            binding,
            rebound: false,
            procedure: false,
        })
    }

    /// The answer of `semantics`'s declared route over `inputs`, lifted
    /// over one finite input: a registry-owned evaluator, the expression
    /// engine over the analysis services, or a declared implementation.
    /// `binding` is the head the answer rests on, and `nested` the policy the
    /// expression engine admits nested invocations under.
    fn route_answer(
        &self,
        semantics: &dyn CommandSemantics,
        inputs: &dyn AnalysisInputs,
        (binding, nested): (&BindingIdentity, NestedPolicy),
    ) -> LiftedAnswer {
        match semantics.route() {
            EvalRoute::Direct { id } => {
                self.enter_direct();
                match id.owner() {
                    EvaluatorOwner::Registry => {
                        evaluate_lifted(semantics, inputs, &mut self.budget(), MAX_CONSTSET_SIZE)
                    }
                }
            }
            EvalRoute::Expression { language } => {
                self.enter_expression();
                // A pack's arity or option row can switch the declared
                // route off; the engine adapter never reads the
                // declaration, so the driver asks it first.
                match semantics
                    .as_declared()
                    .and_then(|declared| declared.invocation_decline(inputs))
                {
                    Some(EvalAnswer::Pending) => LiftedAnswer::Pending,
                    Some(EvalAnswer::Declined(reason)) => LiftedAnswer::Declined(reason),
                    Some(EvalAnswer::Evaluated(_)) | None => {
                        let expression = ExpressionEvaluation {
                            expression: Expression::Assembled(ExpressionRoute { language }),
                            policy: self.policy,
                            nested,
                            head: Some(binding.clone()),
                        };
                        evaluate_lifted(&expression, inputs, &mut self.budget(), MAX_CONSTSET_SIZE)
                    }
                }
            }
            EvalRoute::None { reason } => LiftedAnswer::Declined(DeclineReason::NoRoute(reason)),
            // The specialisation's `evaluate` resolves the declared inputs
            // and runs the body in this thread's host, on the bindings it
            // declares it rests on.
            EvalRoute::Implementation(capability) => {
                if self.unheld_binding(&capability).is_some() {
                    return LiftedAnswer::Declined(DeclineReason::RebindingSuspected);
                }
                self.enter_implementation();
                evaluate_lifted(semantics, inputs, &mut self.budget(), MAX_CONSTSET_SIZE)
            }
        }
    }

    /// `answer` with each evaluated outcome's writes placed, in execution
    /// order: the writes its substitutions made, then its own ordered
    /// stores, each resolved to its place after the structural check a
    /// statement's outcome passes ([`validate_outcome`]). An outcome with a
    /// store that names no place the state can name — a computed one, an
    /// element beside its array, a traced place — is `StatefulNested`.
    fn placed_answer(
        &self,
        semantics: &dyn CommandSemantics,
        inputs: &dyn AnalysisInputs,
        answer: LiftedAnswer,
    ) -> (LiftedAnswer, Vec<Vec<(PlaceRef, StoreOutcome)>>) {
        let LiftedAnswer::Evaluated(outcomes) = &answer else {
            return (answer, Vec::new());
        };
        let plan = semantics.structure(inputs);
        let targets = semantics.store_targets(inputs);
        let mut writes = Vec::with_capacity(outcomes.len());
        for outcome in outcomes {
            let own = validate_outcome(&plan, &targets, outcome).and_then(|()| {
                self.placed_stores(outcome, inputs)
                    .map_err(|_| DeclineReason::StatefulNested)
            });
            match own {
                Ok(own) => {
                    let mut all = outcome.nested_writes.clone();
                    all.extend(own.into_iter().map(|(place, store)| (place, store.clone())));
                    writes.push(all);
                }
                Err(reason) => return (LiftedAnswer::Declined(reason), Vec::new()),
            }
        }
        (answer, writes)
    }

    /// The nested-substitution service: `script` under `state`'s policy, its
    /// binding and every binding its answer rests on recorded in the state's
    /// evidence. A protected script ([`NestedPolicy::Protected`]) runs every
    /// command it holds ([`Self::protected_script`]); under any other policy
    /// the script is the one command a substitution holds
    /// ([`Self::nested_command`]).
    fn nested_answer<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &self,
        script: &str,
        state: &mut EvaluationState,
        from: &LatticeInputs<'_, S1, S2>,
    ) -> EvalAnswer {
        let depth = self.nesting.get();
        if depth >= Budget::EVALUATION_DEPTH {
            return EvalAnswer::Declined(DeclineReason::Budget(BudgetLimit::Depth));
        }
        self.nesting.set(depth + 1);
        let answer = match state.policy {
            NestedPolicy::Protected => self.protected_script(script, state, from),
            NestedPolicy::EffectFreeOnly | NestedPolicy::LocalWrites => {
                self.nested_command(script, state, from)
            }
        };
        self.nesting.set(depth);
        answer
    }

    /// The one command of `script` under `state`'s policy. The command reads
    /// what the state and `from`'s enclosing evaluations have written first.
    fn nested_command<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &self,
        script: &str,
        state: &mut EvaluationState,
        from: &LatticeInputs<'_, S1, S2>,
    ) -> EvalAnswer {
        let prior: Vec<(PlaceRef, StoreOutcome)> = from
            .prior_writes
            .iter()
            .chain(&state.writes)
            .cloned()
            .collect();
        let Some(run) = self.run_script(
            script,
            (from.uses, from.values, from.ssa),
            (prior, state.policy),
        ) else {
            return EvalAnswer::Declined(DeclineReason::Unsupported);
        };
        match self.settle(run, state) {
            Ok(outcome) => EvalAnswer::Evaluated(outcome),
            Err(answer) => answer,
        }
    }

    /// A protected script: every command it holds, in order, under one state
    /// — each as under [`NestedPolicy::LocalWrites`] — until one completes
    /// other than normally, which is the script's completion, with the writes
    /// that ran before it in the state. The result is the last command's, the
    /// empty one for a script with no command. A command that declines, a
    /// value that is not exact and a script that does not parse decline the
    /// whole: the completion of a script is the first of its commands that
    /// does not complete normally, and each must be known to be past. So must
    /// each store a command made: a route takes a store the analysis does not
    /// prove failing as the normal completion's, which a statement's normal
    /// path may, but a script whose completion is observed declines where a
    /// store's place is not proven to take it ([`proved_stores`]). A command
    /// whose plan iterates runs as its plan says ([`Self::protected_loop`]).
    ///
    /// An error a command's own word raises is not read here: that command
    /// declines, and the flag a statement's word raising sets is the
    /// statement's, kept as it was.
    fn protected_script<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &self,
        script: &str,
        state: &mut EvaluationState,
        from: &LatticeInputs<'_, S1, S2>,
    ) -> EvalAnswer {
        let commands =
            crate::segmenter::segment_commands_with_offset_and_config(script, 0, self.lexer_config);
        if commands.iter().any(|seg| seg.is_partial) {
            return EvalAnswer::Declined(DeclineReason::Unsupported);
        }
        let outer_word_error = self.word_error.replace(false);
        let answer = self.run_protected(&commands, script, state, from);
        self.word_error.set(outer_word_error);
        answer
    }

    fn run_protected<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &self,
        commands: &[crate::segmenter::SegmentedCommand],
        script: &str,
        state: &mut EvaluationState,
        from: &LatticeInputs<'_, S1, S2>,
    ) -> EvalAnswer {
        let mut result = ExactValueOrUnavailable::exact_text("");
        let mut completion = CompletionOutcome::Normal;
        for seg in commands {
            // The command's own text: a head the source writes as the
            // command's name, which a `;` may follow (`break;`).
            let at = usize::try_from(seg.span.start()).unwrap_or(usize::MAX);
            let end = usize::try_from(seg.span.end()).unwrap_or(usize::MAX);
            if script.get(at..end).map(|text| split_head(text).0) != Some(seg.name()) {
                return EvalAnswer::Declined(DeclineReason::Unsupported);
            }
            let before = state.writes.len();
            let (ended, value) = match self.protected_loop(seg, state, from) {
                Some(Ok(ran)) => ran,
                Some(Err(answer)) => return answer,
                None => {
                    let prior: Vec<(PlaceRef, StoreOutcome)> = from
                        .prior_writes
                        .iter()
                        .chain(&state.writes)
                        .cloned()
                        .collect();
                    let Some(run) = self.run_command(
                        seg,
                        (from.uses, from.values, from.ssa),
                        (prior, NestedPolicy::LocalWrites),
                    ) else {
                        return EvalAnswer::Declined(DeclineReason::Unsupported);
                    };
                    match self.settle(run, state) {
                        Ok(outcome) => (outcome.completion, outcome.result),
                        Err(answer) => return answer,
                    }
                }
            };
            if let Err(answer) = proved_stores(&state.writes, before, from) {
                return answer;
            }
            match ended {
                CompletionOutcome::Normal => result = value,
                CompletionOutcome::Error {
                    message,
                    error_code,
                    ..
                } => {
                    completion = CompletionOutcome::Error {
                        written: state.writes.len(),
                        message,
                        error_code,
                    };
                    break;
                }
                code @ CompletionOutcome::Code { .. } => {
                    completion = code;
                    break;
                }
            }
        }
        EvalAnswer::Evaluated(Box::new(InvocationOutcome {
            completion,
            result,
            nested_writes: Vec::new(),
            ordered_stores: Vec::new(),
            types: TypeFacts::default(),
            evidence: DependencyEvidence::default(),
        }))
    }

    /// A command of a protected script whose plan iterates — `foreach`,
    /// `lmap`, a pack's loop — run as its plan says ([`Self::run_loop`]), with
    /// its head's binding recorded in the state's evidence: the loop's
    /// completion and result. `None` for a command no iteration plan
    /// describes, which runs as any command does, and for one whose words are
    /// not text the source spells: a word's own writes run before the loop,
    /// and the runner orders none of them.
    fn protected_loop<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &self,
        seg: &crate::segmenter::SegmentedCommand,
        state: &mut EvaluationState,
        from: &LatticeInputs<'_, S1, S2>,
    ) -> Option<Result<(CompletionOutcome, ExactValueOrUnavailable), EvalAnswer>> {
        let head = seg.name();
        if !self.trusted(head) {
            return None;
        }
        let cooked = self.cooked_args(seg);
        if !cooked.iter().all(|arg| {
            matches!(
                arg.source,
                OperandSource::BracedLiteral
                    | OperandSource::Literal
                    | OperandSource::QuotedLiteral
            )
        }) {
            return None;
        }
        let texts: Vec<&str> = cooked.iter().map(|arg| arg.text.as_ref()).collect();
        let words: Vec<InvocationWord<'_>> = cooked.iter().map(ArgWord::word).collect();
        let resolved = self.resolve(head, &words)?;
        let semantics = resolved.semantics.value.semantics()?;
        let inputs = LatticeInputs {
            driver: self,
            prior_writes: from
                .prior_writes
                .iter()
                .chain(&state.writes)
                .cloned()
                .collect(),
            words: Words::Independent,
            view: view_of(&resolved, &texts, &words, InvocationLayout::Source),
            uses: from.uses,
            values: from.values,
            ssa: from.ssa,
            sources: cooked.iter().map(|arg| arg.source).collect(),
        };
        let PlanAnswer::Iterate(plan) = semantics.structure(&inputs) else {
            return None;
        };
        record_binding(
            &mut state.evidence,
            binding_of(head, resolved.canonical_command),
        );
        let depth = self.nesting.get();
        if depth >= Budget::EVALUATION_DEPTH {
            return Some(Err(EvalAnswer::Declined(DeclineReason::Budget(
                BudgetLimit::Depth,
            ))));
        }
        self.nesting.set(depth + 1);
        let ran = self.run_loop(&plan, &inputs, state, from);
        self.nesting.set(depth);
        Some(ran)
    }

    /// The loop `plan` describes over `inputs`, its iterations run in
    /// `state`: each binder written in order, padded with the empty string
    /// past the list's end, then the body as a protected script, whose
    /// completion the plan's rule reads ([`IterationPlan::step`]) — the next
    /// iteration, the loop's end, or the loop's own completion — and the
    /// loop's result is what its result rule makes of what the iterations
    /// yielded ([`LoopResult::of`](tcl_registry::value_transfer::LoopResult::of)).
    /// No iteration binds nothing. Each iteration is charged to the
    /// evaluation's budget by the length of its body.
    ///
    /// A plan the runner cannot follow declines: an iterable that is not a
    /// list the analysis knows exactly, a body that is not brace-quoted text
    /// run in the current frame, a binder that is not a scalar place the
    /// state owns, an exit other than the iterable's exhaustion, and binders
    /// bound on the zero-iteration path. A binder is a scalar store, taken
    /// where the script's stores are ([`held_before`]): one the analysis
    /// proves an array raises an error after the writes before it, with its
    /// message and `-errorcode` unproven, and one whose kind it does not
    /// prove declines.
    fn run_loop<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &self,
        plan: &IterationPlan,
        inputs: &LatticeInputs<'_, S1, S2>,
        state: &mut EvaluationState,
        from: &LatticeInputs<'_, S1, S2>,
    ) -> Result<(CompletionOutcome, ExactValueOrUnavailable), EvalAnswer> {
        let declined = |reason| Err(EvalAnswer::Declined(reason));
        let (Some(body), IterableKind::List(list), ExitRule::Exhaustion, false) = (
            &plan.body,
            &plan.iterable,
            plan.exit,
            plan.zero_iterations_bind,
        ) else {
            return declined(DeclineReason::Unsupported);
        };
        if body.frame != FrameLevel::Relative(0) || plan.binders.is_empty() {
            return declined(DeclineReason::Unsupported);
        }
        let places = plan
            .binders
            .iter()
            .map(|binder| self.binder_place(binder, inputs))
            .collect::<Result<Vec<_>, _>>()
            .map_err(EvalAnswer::Declined)?;
        let text = match inputs.operand(*list, FactDomain::ExactValue) {
            FactView::Exact(value, _) => value.as_str().map_err(EvalAnswer::Declined)?.to_owned(),
            FactView::Pending => return Err(EvalAnswer::Pending),
            FactView::Top(reason) => return declined(reason),
            FactView::Finite(..) | FactView::Domain(_) => return declined(DeclineReason::NotExact),
        };
        // A list that does not parse is the command's error, which the
        // runner does not word.
        let Ok(items) = self.policy.word_rules.split_list(&text) else {
            return declined(DeclineReason::Unsupported);
        };
        let script = inputs.body(body.body).map_err(EvalAnswer::Declined)?.script;
        let mut budget = self.budget();
        let cost = u64::try_from(script.len()).unwrap_or(u64::MAX).max(1);
        let mut yielded = Vec::new();
        for iteration in 0..items.len().div_ceil(places.len()) {
            budget.charge_work(cost).map_err(EvalAnswer::Declined)?;
            for (at, place) in places.iter().enumerate() {
                // A binder takes its element as any store of the script does
                // ([`held_before`]): a proven array raises there.
                if held_before(place.base(), &state.writes, from)? == Held::Array {
                    return Ok((
                        CompletionOutcome::error_unproven(state.writes.len()),
                        ExactValueOrUnavailable::unproven_string(),
                    ));
                }
                let value = items
                    .get(iteration * places.len() + at)
                    .map_or("", AsRef::as_ref);
                state.writes.push((
                    place.clone(),
                    StoreOutcome::Write {
                        target: TargetId(OperandId(0)),
                        value: ExactValue::from_literal(value),
                    },
                ));
            }
            let ran = match self.protected_script(&script, state, from) {
                EvalAnswer::Evaluated(ran) => ran,
                other => return Err(other),
            };
            match plan.step(&ran.completion) {
                LoopStep::Next => yielded.push(ran.result),
                LoopStep::Skip => {}
                LoopStep::Exit => break,
                LoopStep::Leave => return Ok((ran.completion, ran.result)),
            }
        }
        let target = TargetSemantics::of(self.context.profile);
        Ok((CompletionOutcome::Normal, plan.result.of(&yielded, &target)))
    }

    /// The place a loop's binder names: a scalar the state owns, named by
    /// the plan or by an operand. A qualified name, an element and a place
    /// the state cannot own (traced or escaping) are refused.
    fn binder_place<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &self,
        binder: &tcl_registry::value_transfer::Binder,
        inputs: &LatticeInputs<'_, S1, S2>,
    ) -> Result<PlaceRef, DeclineReason> {
        if binder.kind != BindingKind::Scalar {
            return Err(DeclineReason::Unsupported);
        }
        let place = match &binder.name {
            BinderName::Declared(name) if !name.contains("::") => place_named(name),
            BinderName::Declared(_) => return Err(DeclineReason::Unsupported),
            BinderName::Operand(id) => inputs.place(*id)?,
        };
        if place.is_element() || place.name.contains("::") {
            return Err(DeclineReason::Unsupported);
        }
        if !self.state_owns(&place) {
            return Err(DeclineReason::StatefulNested);
        }
        Ok(place)
    }

    /// What a command run for `state` leaves in it: its binding and every
    /// binding its answer rests on recorded in the state's evidence, and its
    /// writes applied to the state in order — when the policy admits them —
    /// so the next read sees them. An outcome that differs between the
    /// members of a finite input declines as correlated: the host evaluation
    /// holds one value per operand.
    ///
    /// Under [`NestedPolicy::EffectFreeOnly`] only an effect-free outcome is
    /// admitted — any store is `StatefulNested`. Under the other policies the
    /// outcome's writes are applied when the state can own every place it
    /// writes ([`Self::state_owns`]) and its completion says how many ran
    /// ([`Self::admitted_writes`]).
    fn settle(
        &self,
        run: ScriptRun,
        state: &mut EvaluationState,
    ) -> Result<Box<InvocationOutcome>, EvalAnswer> {
        record_binding(&mut state.evidence, run.binding);
        let outcomes = match run.answer {
            LiftedAnswer::Pending => return Err(EvalAnswer::Pending),
            LiftedAnswer::Declined(reason) => return Err(EvalAnswer::Declined(reason)),
            LiftedAnswer::Evaluated(outcomes) => outcomes,
        };
        if state.policy == NestedPolicy::EffectFreeOnly
            && outcomes.iter().any(|outcome| outcome.has_stores())
        {
            return Err(EvalAnswer::Declined(DeclineReason::StatefulNested));
        }
        // A run that placed no writes (an effect-free one) has none per
        // outcome.
        let mut placed = run.writes.into_iter();
        let mut members = outcomes
            .into_iter()
            .map(|outcome| (outcome, placed.next().unwrap_or_default()));
        let Some((first, first_writes)) = members.next() else {
            return Err(EvalAnswer::Declined(DeclineReason::Unsupported));
        };
        if members.any(|(outcome, writes)| {
            outcome.result != first.result
                || outcome.completion != first.completion
                || writes != first_writes
        }) {
            return Err(EvalAnswer::Declined(DeclineReason::CorrelatedSets));
        }
        if state.policy != NestedPolicy::EffectFreeOnly {
            match self.admitted_writes(&first_writes, &first) {
                Ok(ran) => state.writes.extend(first_writes.into_iter().take(ran)),
                Err(reason) => return Err(EvalAnswer::Declined(reason)),
            }
        }
        for binding in &first.evidence.bindings {
            record_binding(&mut state.evidence, binding.clone());
        }
        Ok(first)
    }

    /// How many of a nested outcome's `placed` writes the ordered state
    /// applies, or why it cannot apply them: a write the state can own
    /// (`Write`, `WriteElement`, `WriteUnavailable`) and a preserve, which
    /// leaves its place as it was; nothing else. The completion is exact when
    /// it is the normal one, which ran every write, an error, which ran the
    /// first `written` of them, or a code, whose command stores nothing of
    /// its own, so every write is its words'.
    ///
    /// # Errors
    ///
    /// `StatefulNested` for a write to a place the state cannot own, a
    /// may-write or an unbind, and a code completion that stores.
    fn admitted_writes(
        &self,
        placed: &[(PlaceRef, StoreOutcome)],
        outcome: &InvocationOutcome,
    ) -> Result<usize, DeclineReason> {
        let ran = match &outcome.completion {
            CompletionOutcome::Normal => placed.len(),
            CompletionOutcome::Error { written, .. } => (*written).min(placed.len()),
            CompletionOutcome::Code { .. } if outcome.ordered_stores.is_empty() => placed.len(),
            CompletionOutcome::Code { .. } => return Err(DeclineReason::StatefulNested),
        };
        for (place, store) in &placed[..ran] {
            let admitted = match store {
                StoreOutcome::Preserve { .. } => true,
                StoreOutcome::Write { .. }
                | StoreOutcome::WriteElement { .. }
                | StoreOutcome::WriteUnavailable { .. } => self.state_owns(place),
                StoreOutcome::Unbind { .. } | StoreOutcome::MayWrite { .. } => false,
            };
            if !admitted {
                return Err(DeclineReason::StatefulNested);
            }
        }
        Ok(ran)
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
    ) -> DefValues {
        let head = command_binding.map_or("expr", |binding| binding.name.as_str());
        if self.folds.is_some() && !self.trusted(head) {
            self.explain(head, None, "declined: rebinding-suspected".to_owned());
            return DefValues::Each(LatticeValue::Overdefined, None);
        }
        self.enter_expression();
        let expression = ExpressionEvaluation {
            expression: Expression::Parsed(expr),
            policy: self.policy,
            nested: NestedPolicy::EffectFreeOnly,
            head: Some(binding_of(
                head,
                command_binding.map_or("expr", |binding| binding.identity.as_str()),
            )),
        };
        let answer = self.evaluate_expression_at(&expression, uses, values, ssa);
        self.explain(head, Some(expression.route()), answer_label(&answer));
        if let LiftedAnswer::Evaluated(outcomes) = &answer {
            self.explain_paths(outcome_paths(outcomes, None));
        }
        if self.raises_end_paths()
            && let Some(raised) = Self::raised_expression(&answer)
        {
            return DefValues::Raised(Box::new(raised));
        }
        let value = match answer {
            LiftedAnswer::Pending => LatticeValue::Unknown,
            LiftedAnswer::Declined(_) => LatticeValue::Overdefined,
            LiftedAnswer::Evaluated(outcomes) => lattice_of_outcomes(&outcomes, result_of),
        };
        DefValues::Each(value, None)
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
            nested: NestedPolicy::EffectFreeOnly,
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
        let inputs = self.expression_inputs((uses, values, ssa));
        evaluate_lifted(expression, &inputs, &mut self.budget(), MAX_CONSTSET_SIZE)
    }
}

static EMPTY_NAMES: BTreeSet<String> = BTreeSet::new();

/// One statement of a function unit: the block it sits in and its index
/// there, in the CFG block and the SSA block alike.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StatementId {
    /// The block.
    pub block: crate::cfg::BlockId,
    /// The statement's index in the block.
    pub index: usize,
}

/// The exact value a word of a statement has at that statement, with its
/// folded type — the lattice's answer, never a token relabelled as a
/// literal. `word` counts the call's words from the command's own (`0`,
/// never an operand here): a literal word is its text, and a substituted
/// word its literal runs and variable reads, each read at the statement's
/// use version, concatenated. A word the lattice holds no exact value for
/// — a command substitution, a read of a finite set or an unknown value,
/// an expansion, a word respelled after lowering — is `None`, as is any
/// word of a statement the solver never reached. Only a whole-word
/// variable read carries a folded type, the one its definition states.
/// A read of a name the statement writes is `None` once one of its words'
/// command substitutions has run: the substitution may have written it,
/// and the statement holds the one use version from before
/// (`set i 9; lindex $l [set i 0] $i` reads `$i` as 0).
/// `config` is the document's grammar, which the literal runs are decoded
/// under: the unit does not keep one.
#[must_use]
pub fn proven_word_value(
    fu: &crate::compilation_unit::FunctionUnit,
    statement: StatementId,
    word: usize,
    config: LexerConfig,
) -> Option<(ExactValue, Option<FoldedType>)> {
    if !fu.sccp.executable_blocks.contains(&statement.block) {
        return None;
    }
    let call = fu
        .cfg
        .blocks
        .get(&statement.block)?
        .statements
        .get(statement.index)?;
    let at = fu
        .ssa
        .blocks
        .get(&statement.block)?
        .statements
        .get(statement.index)?;
    let Statement::Call { args, tokens, .. } = call else {
        return None;
    };
    let arguments = call_arguments(args, tokens.as_ref(), &config);
    let argument = arguments.get(word.checked_sub(1)?)?;
    let written = if at.defs.is_empty() {
        HashSet::new()
    } else {
        let start = tokens
            .as_ref()
            .and_then(|tokens| tokens.argv.get(word))
            .map(|span| span.start());
        written_before(
            at,
            start,
            &crate::word_subst::lifted_calls(tokens.as_ref(), config),
        )
    };
    proven_argument(argument, &at.uses, &written, fu, config)
}

/// Where a command substitution runs: in a statement's words, or in a
/// block's terminator — a `return` value or a branch condition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SubstitutionHost {
    /// The words of a statement.
    Statement(StatementId),
    /// The terminator of a block.
    Terminator(crate::cfg::BlockId),
}

/// The command substitutions `host` performs, innermost first
/// ([`crate::word_subst`]), which [`proven_substituted_word_value`]
/// addresses: none for a host the solver never reached, a statement with
/// no words of its own, or a branch condition with no source offset.
#[must_use]
pub fn substitution_calls(
    fu: &crate::compilation_unit::FunctionUnit,
    host: SubstitutionHost,
    config: LexerConfig,
    surface: &tcl_registry::model::DocumentCommandSurface<'_>,
) -> Vec<crate::word_subst::LiftedCall> {
    let block = match host {
        SubstitutionHost::Statement(id) => id.block,
        SubstitutionHost::Terminator(block) => block,
    };
    if !fu.sccp.executable_blocks.contains(&block) {
        return Vec::new();
    }
    let Some(cfg_block) = fu.cfg.blocks.get(&block) else {
        return Vec::new();
    };
    match host {
        SubstitutionHost::Statement(id) => match cfg_block.statements.get(id.index) {
            Some(
                Statement::Call {
                    tokens: Some(tokens),
                    ..
                }
                | Statement::AssignValue {
                    tokens: Some(tokens),
                    ..
                },
            ) if tokens.synthetic.is_none() => {
                crate::word_subst::lifted_calls(Some(tokens), config)
            }
            _ => Vec::new(),
        },
        SubstitutionHost::Terminator(_) => match &cfg_block.terminator {
            Some(crate::cfg::Terminator::Return { value_word, .. }) => {
                crate::word_subst::lifted_calls_in_word(value_word.as_ref(), config, surface)
            }
            Some(crate::cfg::Terminator::Branch {
                condition,
                condition_base: Some(base),
                ..
            }) => crate::word_subst::lifted_calls_in_expr(condition, Some(*base), config, surface),
            _ => Vec::new(),
        },
    }
}

/// The words of the command `block`'s `return` terminator stands for, as
/// the segmenter recovers them from the document `source` at the
/// terminator's span, each span an offset into `source`: `None` for a block
/// the solver never reached, any other terminator, a terminator with no
/// source span, or a span that does not hold exactly one complete command.
#[must_use]
pub fn return_command_tokens(
    fu: &crate::compilation_unit::FunctionUnit,
    block: crate::cfg::BlockId,
    source: &str,
    config: LexerConfig,
) -> Option<crate::ir::CommandTokens> {
    if !fu.sccp.executable_blocks.contains(&block) {
        return None;
    }
    let Some(crate::cfg::Terminator::Return {
        span: Some(span), ..
    }) = &fu.cfg.blocks.get(&block)?.terminator
    else {
        return None;
    };
    let span = fu.abs_span(*span);
    let text = source.get(span.start() as usize..span.end() as usize)?;
    let segments =
        crate::segmenter::segment_commands_with_offset_and_config(text, span.start(), config);
    let [segment] = segments.as_slice() else {
        return None;
    };
    if segment.is_partial {
        return None;
    }
    let map = tcl_lexer::SourceMap::new(text).with_base(span.start(), 0, 0);
    let tokens = crate::ir::CommandTokens::from_segmented(&map, config, segment);
    (!tokens.argv.is_empty()).then_some(tokens)
}

/// [`proven_word_value`] for word `word` of the `return` command `tokens`
/// holds ([`return_command_tokens`]), the terminator of `block`: read at the
/// block's exit versions, and only for a command that performs no command
/// substitution, since the exit versions hold whatever one writes.
#[must_use]
pub fn proven_return_word_value(
    fu: &crate::compilation_unit::FunctionUnit,
    block: crate::cfg::BlockId,
    tokens: &crate::ir::CommandTokens,
    word: usize,
    config: LexerConfig,
) -> Option<(ExactValue, Option<FoldedType>)> {
    if !fu.sccp.executable_blocks.contains(&block)
        || !crate::word_subst::lifted_calls(Some(tokens), config).is_empty()
    {
        return None;
    }
    let versions = &fu.ssa.blocks.get(&block)?.exit_versions;
    let arguments = call_arguments(tokens.argv_texts.get(1..)?, Some(tokens), &config);
    proven_argument(
        arguments.get(word.checked_sub(1)?)?,
        versions,
        &HashSet::new(),
        fu,
        config,
    )
}

/// [`proven_word_value`] for word `word` of `calls[call]`, a command
/// substitution `host` performs, where `calls` is what
/// [`substitution_calls`] lifts there. A statement's substitution reads at
/// the statement's use versions, under the same rule for a name the
/// statement writes; a terminator's reads at its block's exit versions,
/// which the definition point ahead of it has already given every name its
/// words write.
#[must_use]
pub fn proven_substituted_word_value(
    fu: &crate::compilation_unit::FunctionUnit,
    host: SubstitutionHost,
    calls: &[crate::word_subst::LiftedCall],
    (call, word): (usize, usize),
    config: LexerConfig,
) -> Option<(ExactValue, Option<FoldedType>)> {
    let tokens = calls.get(call)?.words.as_ref()?;
    let start = tokens.argv.get(word)?.start();
    let (versions, written) = match host {
        SubstitutionHost::Statement(id) => {
            if !fu.sccp.executable_blocks.contains(&id.block) {
                return None;
            }
            let at = fu.ssa.blocks.get(&id.block)?.statements.get(id.index)?;
            (&at.uses, written_before(at, Some(start), calls))
        }
        SubstitutionHost::Terminator(block) => {
            if !fu.sccp.executable_blocks.contains(&block) {
                return None;
            }
            (&fu.ssa.blocks.get(&block)?.exit_versions, HashSet::new())
        }
    };
    let arguments = call_arguments(tokens.argv_texts.get(1..)?, Some(tokens), &config);
    proven_argument(
        arguments.get(word.checked_sub(1)?)?,
        versions,
        &written,
        fu,
        config,
    )
}

/// The names the statement `at` writes, when one of the command
/// substitutions `calls` its words perform has run before the read at
/// `start` (or the read's position is unknown); otherwise none.
fn written_before(
    at: &SsaStatement,
    start: Option<u32>,
    calls: &[crate::word_subst::LiftedCall],
) -> HashSet<Symbol> {
    let ran = start.is_none_or(|start| calls.iter().any(|call| call.span.end() <= start));
    if ran {
        at.defs.keys().copied().collect()
    } else {
        HashSet::new()
    }
}

/// One argument's proven value: a literal is its text, a substituted word
/// [`proven_substitution`] at `versions`, never reading a name in `written`.
fn proven_argument(
    argument: &ArgWord<'_>,
    versions: &HashMap<Symbol, Version>,
    written: &HashSet<Symbol>,
    fu: &crate::compilation_unit::FunctionUnit,
    config: LexerConfig,
) -> Option<(ExactValue, Option<FoldedType>)> {
    match argument.source {
        OperandSource::BracedLiteral | OperandSource::Literal | OperandSource::QuotedLiteral => {
            Some((ExactValue::from_literal(&argument.text), None))
        }
        OperandSource::Substituted => {
            proven_substitution(&argument.text, versions, written, fu, config)
        }
        OperandSource::Unknown => None,
    }
}

/// [`proven_word_value`] of a substituted word: its literal runs decoded,
/// each variable read at the use version `uses` gives it, and nothing else.
fn proven_substitution(
    text: &str,
    uses: &HashMap<Symbol, Version>,
    written: &HashSet<Symbol>,
    fu: &crate::compilation_unit::FunctionUnit,
    config: LexerConfig,
) -> Option<(ExactValue, Option<FoldedType>)> {
    use tcl_lexer::word_parts::{SubstFlags, WordBody, WordPart as Part, decompose};
    let read = |name: &str| -> Option<(ExactValue, ValueKey)> {
        let symbol = fu.ssa.var_symbol(name)?;
        if written.contains(&symbol) {
            return None;
        }
        let key = (symbol, *uses.get(&symbol)?);
        match fu.sccp.values.get(&key)? {
            LatticeValue::Const(value) => Some((const_to_exact(value), key)),
            _ => None,
        }
    };
    if let Some(name) = simple_var_ref_name(text, config.braced_var) {
        let (value, key) = read(name)?;
        return Some((value, fu.sccp.folded_types.get(&key).cloned()));
    }
    let parts = match decompose(text.as_bytes(), SubstFlags::default(), config) {
        WordBody::Literal(bytes) => return Some((exact_of_bytes(bytes.to_vec()), None)),
        WordBody::Parts(parts) => parts,
    };
    let mut bytes = Vec::with_capacity(text.len());
    for part in parts {
        match part {
            Part::Text(run) => bytes.extend_from_slice(&run),
            Part::Variable(reference) => {
                let (value, _) = read(&variable_name(&reference).ok()?)?;
                bytes.extend_from_slice(&value.bytes);
            }
            Part::Command(_) | Part::ParseError(_) => return None,
        }
    }
    Some((exact_of_bytes(bytes), None))
}

/// The inputs of an expression a rewrite evaluates outside a solver run:
/// its `$name` reads answer from `constants`, a nested command declines but
/// where it calls a procedure of `module` ([`detached_procedure_result`]),
/// and a math function asks the driver's binding service under the
/// rewrite's trust stance.
struct DetachedExpressionInputs<'a, 'd> {
    driver: &'a LatticeDriver<'d>,
    view: ResolvedInvocationView<'a>,
    constants: &'a HashMap<String, ExactValue>,
    /// The module's procedures and the function the expression runs in,
    /// where a re-run's exit reading holds them.
    module: Option<(&'a crate::interprocedural::ModuleProcedures<'a>, &'a str)>,
    /// The stance the expression is read under.
    stance: (BuiltinFoldInputs<'a>, FoldPolicy),
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

    fn nested(&self, script: &str, _state: &mut EvaluationState) -> EvalAnswer {
        let result = self.module.and_then(|module| {
            detached_procedure_result(module, script, self.constants, self.stance)
        });
        match result {
            Some(value) => EvalAnswer::Evaluated(Box::new(InvocationOutcome {
                completion: CompletionOutcome::Normal,
                result: ExactValueOrUnavailable::Exact(value),
                nested_writes: Vec::new(),
                ordered_stores: Vec::new(),
                types: TypeFacts::default(),
                evidence: DependencyEvidence::default(),
            })),
            None => EvalAnswer::Declined(DeclineReason::Unsupported),
        }
    }

    fn math_function(&self, name: &str) -> Result<BindingIdentity, DeclineReason> {
        self.driver.math_function(name)
    }

    fn context(&self) -> &AnalysisContext {
        &self.driver.context
    }
}

/// The result of the one command `script` holds, outside a solver run, where
/// it calls a procedure of the module that names no place and writes none
/// outside its frame: a re-run of the callee under the call's arguments
/// ([`crate::interprocedural::ModuleProcedures::rerun`]). Each argument is a
/// literal word, a variable `constants` holds, or a command substitution
/// read the same way — or one on the expression route whose one word is a
/// literal expression. `None` wherever any of that does not hold, and for a
/// callee that does not complete normally.
fn detached_procedure_result(
    (module, function): (&crate::interprocedural::ModuleProcedures<'_>, &str),
    script: &str,
    constants: &HashMap<String, ExactValue>,
    (folds, policy): (BuiltinFoldInputs<'_>, FoldPolicy),
) -> Option<ExactValue> {
    let config = module.lexer_config();
    let commands = crate::segmenter::segment_commands_with_offset_and_config(script, 0, config);
    let [seg] = commands.as_slice() else {
        return None;
    };
    if split_head(script).0 != seg.name() {
        return None;
    }
    let callee = module.resolve(seg.name(), function, folds.trust)?;
    if !module.keeps_to_its_frame(&callee) {
        return None;
    }
    let arguments: Vec<Option<ExactValue>> = seg
        .arg_tokens()
        .iter()
        .zip(seg.arg_single_token())
        .zip(seg.args())
        .map(|((token, &single), text)| {
            detached_word(
                (module, function),
                (text, token.kind, single),
                constants,
                (folds, policy),
            )
        })
        .collect();
    let params = module.parameter_values(&callee, &arguments, folds.dialect)?;
    let rerun = module.rerun(
        &callee,
        (&params, &[]),
        crate::interprocedural::RerunStance { policy, folds },
    )?;
    if !rerun.completes {
        return None;
    }
    rerun.result.clone()
}

/// One argument word of a command [`detached_procedure_result`] reads.
fn detached_word(
    (module, function): (&crate::interprocedural::ModuleProcedures<'_>, &str),
    (text, kind, single): (&str, TokenType, bool),
    constants: &HashMap<String, ExactValue>,
    (folds, policy): (BuiltinFoldInputs<'_>, FoldPolicy),
) -> Option<ExactValue> {
    use tcl_lexer::word_parts::{SubstFlags, WordBody, WordPart as Part, decompose};
    let config = module.lexer_config();
    if single && let Some(value) = literal_token_value(text, kind, &config) {
        return Some(ExactValue::from_literal(&value));
    }
    if let Some(name) = simple_var_ref_name(text, config.braced_var) {
        return constants.get(name).cloned();
    }
    let WordBody::Parts(parts) = decompose(text.as_bytes(), SubstFlags::default(), config) else {
        return None;
    };
    let [Part::Command(script)] = parts.as_slice() else {
        return None;
    };
    let inner = std::str::from_utf8(script).ok()?;
    if let Some(value) =
        detached_procedure_result((module, function), inner, constants, (folds, policy))
    {
        return Some(value);
    }
    let expression = detached_expression_word(module.registry(), inner, folds, config)?;
    let grammar = folds
        .dialect
        .map_or_else(tcl_dialect::LexerGrammar::default, |profile| {
            profile.grammar
        });
    let node = tcl_syntax::expr::parser::parse_expr_with_grammar(&expression, &grammar);
    evaluate_expression_in_module(&node, constants, (folds, policy), Some((module, function)))
}

/// The expression of the one command `script` holds where the command is on
/// the expression route under `folds`' trust and its one word is a literal
/// expression.
fn detached_expression_word(
    registry: &CommandRegistry,
    script: &str,
    folds: BuiltinFoldInputs<'_>,
    config: LexerConfig,
) -> Option<String> {
    let commands = crate::segmenter::segment_commands_with_offset_and_config(script, 0, config);
    let [seg] = commands.as_slice() else {
        return None;
    };
    let head = seg.name();
    let trusted = match folds.trust {
        FoldTrust::WholeModule => folds.mutations.trusts(head),
        FoldTrust::ObservedBindings => folds.mutations.observed_binding_is_the_builtin(head),
    };
    let ([token], [true], [text]) = (seg.arg_tokens(), seg.arg_single_token(), seg.args()) else {
        return None;
    };
    let expression = literal_token_value(text, token.kind, &config)?;
    let words = [InvocationWord::Literal(&expression)];
    let resolved = registry
        .resolve_structured_invocation(
            InvocationWords::structured(InvocationWord::Literal(head), &words),
            registry.own_surface_query(),
        )
        .resolved()?;
    let on_the_route = resolved
        .semantics
        .value
        .semantics()
        .is_some_and(|semantics| matches!(semantics.route(), EvalRoute::Expression { .. }));
    (trusted && on_the_route).then(|| expression.into_owned())
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
    evaluate_expression_in_module(node, constants, (folds, policy), None)
}

/// [`evaluate_expression_detached`] with a command the expression runs that
/// calls a procedure of `module` re-run for its result
/// ([`detached_procedure_result`]): a re-run's exit reading.
pub(crate) fn evaluate_expression_in_module(
    node: &ExprNode,
    constants: &HashMap<String, ExactValue>,
    (folds, policy): (BuiltinFoldInputs<'_>, FoldPolicy),
    module: Option<(&crate::interprocedural::ModuleProcedures<'_>, &str)>,
) -> Option<ExactValue> {
    let driver = LatticeDriver::detached(Some(folds), policy);
    let expression = ExpressionEvaluation {
        expression: Expression::Parsed(node),
        policy,
        nested: NestedPolicy::EffectFreeOnly,
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
            arity: None,
        },
        constants,
        module,
        stance: (folds, policy),
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

/// What a synthetic embedded-substitution call and its host statement leave
/// in their definitions ([`LatticeDriver::evaluate_embedded`]).
pub(crate) struct EmbeddedAnswer {
    /// The call's definitions, one answer each.
    pub(crate) call: Vec<DefAnswer>,
    /// The host's definitions.
    pub(crate) host: DefValues,
    /// How many writes ran before the error, when the pair certainly raises:
    /// the call's definitions are those writes' and the host never runs.
    pub(crate) raised: Option<usize>,
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

/// The versions the host of the embedded call at `index` reads: those it
/// and the call record, with each place the call defines at the version
/// ahead of the call — the host's own records name the call's definitions,
/// and its words that run before the writes read what was there. A place no
/// definition and no root value has ahead of the call is read as unknown
/// rather than waited for.
fn incoming_versions<S: std::hash::BuildHasher>(
    block: &crate::ssa::SsaBlock,
    (index, host): (usize, usize),
    values: &HashMap<ValueKey, LatticeValue, S>,
) -> HashMap<Symbol, Version> {
    let (call, host) = (&block.statements[index], &block.statements[host]);
    let mut incoming: HashMap<Symbol, Version> = host.uses.clone();
    incoming.extend(
        call.uses
            .iter()
            .map(|(&symbol, &version)| (symbol, version)),
    );
    for &symbol in call.defs.keys() {
        let version = crate::sccp::prior_version(block, index, symbol);
        if version == 0 && !values.contains_key(&(symbol, 0)) {
            incoming.remove(&symbol);
        } else {
            incoming.insert(symbol, version);
        }
    }
    incoming
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

/// An expression's outcome before its writes and completion: `result`, no
/// store, and the evidence of the route that computed it.
fn expression_outcome(
    result: ExactValueOrUnavailable,
    route: EvalRoute,
    implementation: &'static str,
    revision: u64,
    bindings: Vec<BindingIdentity>,
) -> InvocationOutcome {
    InvocationOutcome {
        completion: CompletionOutcome::Normal,
        nested_writes: Vec::new(),
        result,
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
    /// Which nested operations the evaluation admits: only effect-free
    /// ones, or writes to places its state can own.
    nested: NestedPolicy,
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
        // The operand words substituted before the engine parses them, and
        // the writes their commands made, come first.
        let mut state = input
            .word_state()
            .unwrap_or_else(|| EvaluationState::new(self.nested));
        if let Some(head) = &self.head {
            record_binding(&mut state.evidence, head.clone());
        }
        let answer = {
            let mut services = ExprServices::new(input, &mut state, budget);
            evaluate_expression(node, &mut services, self.policy)
        };
        let (result, completion) = match answer {
            ExprAnswer::Pending => return EvalAnswer::Pending,
            ExprAnswer::Declined(reason) => return EvalAnswer::Declined(reason),
            ExprAnswer::Value(value) => (
                ExactValueOrUnavailable::Exact(value),
                CompletionOutcome::Normal,
            ),
            // The evaluation ended on a nested error: the writes the state
            // holds are the ones that ran before it, and the outcome has no
            // value.
            ExprAnswer::Ended(ended) => match *ended {
                CompletionOutcome::Error {
                    message,
                    error_code,
                    ..
                } => (
                    ExactValueOrUnavailable::Unavailable(FactBounds {
                        existence: Existence::MayBound,
                        intrep: None,
                        shape: None,
                        segments: None,
                        taint: None,
                    }),
                    CompletionOutcome::Error {
                        written: state.writes.len(),
                        message,
                        error_code,
                    },
                ),
                CompletionOutcome::Normal | CompletionOutcome::Code { .. } => {
                    return EvalAnswer::Declined(DeclineReason::StatefulNested);
                }
            },
        };
        let mut outcome = expression_outcome(
            result,
            self.route(),
            self.identity(),
            ExpressionRoute::REVISION,
            state.evidence.bindings,
        );
        outcome.completion = completion;
        outcome.nested_writes = state.writes;
        outcome.evidence.numerals = self.policy.numbers;
        outcome.evidence.release =
            tcl_registry::value_transfer::TargetSemantics::of(self.policy.dialect).release;
        EvalAnswer::Evaluated(Box::new(outcome))
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

/// The value of a word a case-list statement recorded as `text`, when the
/// source states one: a braced word is its content with backslash-newlines
/// collapsed; a bare or quoted word with nothing to substitute is its escapes
/// decoded under the document's grammar; `None` for a word that substitutes.
/// The one decoder of a recorded word, read by the selection's arguments
/// ([`switch_arguments`]) and by the flattened `switch` chain's operands, so
/// the two cannot read one word two ways: `a\nb` is three characters and
/// `{a\b}` is three characters, whichever way the statement is analysed.
pub(crate) fn recorded_word_value<'t>(
    text: &'t str,
    braced: bool,
    config: &LexerConfig,
) -> Option<Cow<'t, str>> {
    if braced {
        return Some(WordValueRules::from_config(config).collapse_braced_word(text));
    }
    (!substitutes(text)).then(|| tcl_lexer::backslash_subst_in(text, config.escapes))
}

/// Whether a bare or quoted word still has a substitution to make: a `$` or
/// `[` that no backslash escapes, so `a\$b` is the literal `a$b`. A backslash
/// escapes the byte after it, and all three bytes are ASCII, so walking bytes
/// never splits a character.
fn substitutes(text: &str) -> bool {
    let mut bytes = text.bytes();
    while let Some(byte) = bytes.next() {
        match byte {
            b'\\' => {
                bytes.next();
            }
            b'$' | b'[' => return true,
            _ => {}
        }
    }
    false
}

/// What the source says about how an operand's word substitutes, for the
/// inputs that answer its structure ([`AnalysisInputs::word_structure`])
/// and a substituted word's value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OperandSource {
    /// A single braced word: its content is its value.
    BracedLiteral,
    /// A bare word with nothing to substitute — or, in a statement with no
    /// token snapshot, a word taken as spelled: its cooked text is its
    /// value.
    Literal,
    /// A double-quoted word with nothing to substitute: its cooked text is
    /// its value. Apart from [`Self::Literal`] because 9.1b0's byte-compiled
    /// `switch` reads a quoted fall-through body as a command.
    QuotedLiteral,
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
    fn of_token(
        text: &'t str,
        (kind, quoted): (TokenType, bool),
        single: bool,
        config: &LexerConfig,
    ) -> Self {
        if kind == TokenType::Expand {
            return Self::spelled(text, InvocationWordKind::Expanded, OperandSource::Unknown);
        }
        match literal_token_value(text, kind, config).filter(|_| single) {
            Some(value) => Self {
                text: value,
                kind: InvocationWordKind::Literal,
                source: if kind == TokenType::Str {
                    OperandSource::BracedLiteral
                } else if quoted {
                    OperandSource::QuotedLiteral
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
                // A quoted word's span opens at its `"`, so it runs past its
                // content; a bare word's is its spelling.
                let span = tokens.argv[at];
                let quoted = tokens.argv_kinds[at] == TokenType::Esc
                    && (span.end() - span.start()) as usize > text.len();
                ArgWord::of_token(
                    text,
                    (tokens.argv_kinds[at], quoted),
                    tokens.single_token_word[at],
                    config,
                )
            } else {
                ArgWord::spelled(text, InvocationWordKind::Dynamic, OperandSource::Unknown)
            }
        })
        .collect()
}

/// A case-list statement's words as its lowering recorded them
/// (`Statement::Switch`'s `raw_args` with the per-word braced and quoted
/// flags, which the caller has checked cover every word), each read by
/// [`recorded_word_value`]: a word with a value is that value, its source
/// saying whether it was braced, quoted or bare; any other word is
/// substituted from its spelling.
fn switch_arguments<'t>(
    raw_args: &'t [String],
    (braced, quoted): (&[bool], &[bool]),
    config: &LexerConfig,
) -> Vec<ArgWord<'t>> {
    let flag = |flags: &[bool], index: usize| flags.get(index).copied().unwrap_or(false);
    raw_args
        .iter()
        .enumerate()
        .map(|(index, text)| {
            let braced = flag(braced, index);
            let Some(value) = recorded_word_value(text, braced, config) else {
                return ArgWord::spelled(
                    text,
                    InvocationWordKind::Dynamic,
                    OperandSource::Substituted,
                );
            };
            let source = match (braced, flag(quoted, index)) {
                (true, _) => OperandSource::BracedLiteral,
                (false, true) => OperandSource::QuotedLiteral,
                (false, false) => OperandSource::Literal,
            };
            ArgWord {
                text: value,
                kind: InvocationWordKind::Literal,
                source,
            }
        })
        .collect()
}

/// The command a statement's binding site names: the registry
/// identity the lowering recorded at the statement's own span, when every
/// site there names the same one — the canonical command a
/// `Statement::Switch`, which keeps no resolved name of its own, was lowered
/// for.
fn binding_at(cfg: &crate::cfg::Function, span: Span) -> Option<&str> {
    let mut identities = cfg
        .command_binding_sites
        .iter()
        .filter(|site| site.span == span)
        .map(|site| site.binding.identity.as_str());
    let first = identities.next()?;
    identities.all(|other| other == first).then_some(first)
}

/// How many pattern and body pairs a case-list plan reads: the inline
/// form's pairs, or the clause-list word's exact value split under the
/// document's list rules — `None` where that word has no exact value or is
/// not a list of pairs, which the transfer declines too.
fn plan_clause_count(
    inputs: &dyn AnalysisInputs,
    arms: &CaseArms,
    rules: WordValueRules,
) -> Option<usize> {
    match arms {
        CaseArms::Words(pairs) => Some(pairs.len()),
        CaseArms::List(list) => {
            let FactView::Exact(value, _) = inputs.operand(*list, FactDomain::ExactValue) else {
                return None;
            };
            let text = String::from_utf8(value.bytes).ok()?;
            let elements = rules.split_list(&text).ok()?.len();
            elements.is_multiple_of(2).then_some(elements / 2)
        }
    }
}

/// The `Selected` branch fact of each arm of `arms` whose body no member of
/// `record`'s subject runs: `block` holds the statement, the fact's span is
/// the arm's pattern, its condition the pattern's text, its value `false`, and
/// it has no target — no block stands for an arm. An arm that passes its body
/// on with `-` is judged by the body it leads to, so the alternate patterns of
/// a running body are not reported, and the final `default`, which has no
/// pattern, never is.
fn unreached_arm_facts(
    block: &str,
    record: &crate::sccp::SelectionRecord,
    arms: &[crate::ir::SwitchArm],
) -> Vec<crate::sccp::ConstantBranch> {
    if record.fact.selected.is_empty() {
        return Vec::new();
    }
    let body_of = |index: usize| {
        arms[index..]
            .iter()
            .position(|arm| !arm.fallthrough)
            .map_or(arms.len(), |offset| index + offset)
    };
    arms.iter()
        .enumerate()
        .zip(&record.arm_pattern_spans)
        .filter(|((index, _), _)| !record.fact.bodies.contains(&Some(body_of(*index))))
        .map(|((_, arm), span)| crate::sccp::ConstantBranch {
            block: block.to_owned(),
            span: Some(*span),
            condition: arm.pattern.clone(),
            value: false,
            taken_target: String::new(),
            not_taken_target: String::new(),
            kind: crate::sccp::BranchFactKind::Selected,
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
        arity: Some(semantics.arity),
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
/// as, with its classification as the additional fact. A double is spelled
/// as Tcl spells it (`3.0`, `1e+301`), never as Rust does (`3`).
pub(crate) fn const_to_exact(c: &ConstValue) -> ExactValue {
    match c {
        ConstValue::Int(i) => ExactValue::int(*i),
        ConstValue::Float(f) => ExactValue {
            bytes: tcl_syntax::number::format_double(*f).into_bytes(),
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

/// The text a lattice constant renders as, when it is text.
pub(crate) fn const_text(c: &ConstValue) -> Option<String> {
    String::from_utf8(const_to_exact(c).bytes).ok()
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
pub(crate) fn exact_to_lattice(value: &ExactValue) -> LatticeValue {
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
    /// The writes the enclosing evaluations have made before this
    /// invocation runs, in order: a read consults them before the lattice
    /// at the program point (the ordered evaluation state's rule, § `expr`).
    prior_writes: Vec<(PlaceRef, StoreOutcome)>,
    /// How the operand words that substitute are evaluated.
    words: Words,
}

/// How an invocation's substituting operand words are evaluated.
enum Words {
    /// Each word on its own, under the effect-free policy.
    Independent,
    /// Every substituting word once, in operand order, under one ordered
    /// state, evaluated when the first is read: the writes one word's
    /// commands make are seen by the next word and by the route that
    /// assembles them, and a word is evaluated once however often the route
    /// reads it. A route reads every operand before it reads a cell
    /// (`evaluate_lifted` pins them first), so the words have run, in order,
    /// before anything of the command reads what they wrote.
    Ordered(RefCell<Option<Box<OrderedWords>>>),
}

impl Words {
    /// How an invocation's words are evaluated under `policy`: each on its
    /// own where it runs effect-free, else in order under one state.
    fn under(policy: NestedPolicy) -> Self {
        match policy {
            NestedPolicy::EffectFreeOnly => Self::Independent,
            NestedPolicy::LocalWrites | NestedPolicy::Protected => {
                Self::Ordered(RefCell::new(None))
            }
        }
    }

    /// The writes the words made, in order, where they were evaluated under
    /// one ordered state.
    fn writes(&self) -> Vec<(PlaceRef, StoreOutcome)> {
        match self {
            Self::Ordered(evaluated) => evaluated
                .borrow()
                .as_ref()
                .map(|words| words.state.writes.clone())
                .unwrap_or_default(),
            Self::Independent => Vec::new(),
        }
    }
}

/// An invocation's substituting operand words after their evaluation.
struct OrderedWords {
    /// Each operand's fact; `None` for an operand no substitution reaches.
    facts: Vec<Option<FactView>>,
    /// The state the words ran under: its writes are what they stored.
    state: EvaluationState,
}

impl<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher> LatticeInputs<'_, S1, S2> {
    /// Re-resolve the view's roles over the words' lattice values, when the
    /// command's roles depend on its words (`arg_role_resolver`) and a
    /// substituted word kept the resolver from reading them: the command
    /// sees the values, so a subject the lattice proves (`regexp {(a+)b} $s
    /// -> g`, `lassign $l a b`) no longer hides its targets' roles. Only
    /// when every word is exact, and every word the values make a target
    /// is a literal name — a computed name is no place.
    fn resolve_roles_over_values(&mut self, resolved: &ResolvedInvocation<'_, '_>) {
        let semantics = &resolved.semantics;
        let Some(derive_roles) = semantics.arg_role_resolver else {
            return;
        };
        if self
            .view
            .operands
            .iter()
            .all(|operand| operand.kind == InvocationWordKind::Literal)
        {
            return;
        }
        let values: Option<Vec<String>> = (0..self.view.operands.len())
            .map(
                |index| match self.operand(OperandId(index), FactDomain::ExactValue) {
                    FactView::Exact(value, _) => String::from_utf8(value.bytes).ok(),
                    _ => None,
                },
            )
            .collect();
        let Some(values) = values else {
            return;
        };
        let words: Vec<&str> = values.iter().map(String::as_str).collect();
        let offset = semantics.argument_offset;
        let Some(arguments) = words.get(offset..) else {
            return;
        };
        let roles = derive_roles(arguments);
        let role_at = |index: usize| {
            index
                .checked_sub(offset)
                .and_then(|i| u8::try_from(i).ok())
                .and_then(|i| roles.iter().find(|(at, _)| *at == i).map(|(_, role)| *role))
        };
        let names_are_literal = self
            .view
            .operands
            .iter()
            .enumerate()
            .all(|(index, operand)| {
                role_at(index) != Some(ArgRole::VarWrite)
                    || operand.kind == InvocationWordKind::Literal
            });
        if !names_are_literal {
            return;
        }
        for (index, operand) in self.view.operands.iter_mut().enumerate() {
            operand.role = role_at(index);
        }
    }

    /// The lattice fact for `name` at this statement's use version, after
    /// the writes the enclosing evaluations made: the last of them decides
    /// what the name holds.
    fn named_fact(&self, name: &str) -> FactView {
        match written_in(&self.prior_writes, name) {
            WrittenPlace::Exact(value) => return FactView::Exact(value, None),
            WrittenPlace::Unknown => return FactView::Top(DeclineReason::NotExact),
            WrittenPlace::Untouched => {}
        }
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

    /// What the writes the invocation's own substituting words made say of
    /// `name`: Tcl substitutes every word before the command runs, so the
    /// command's reads come after them.
    fn words_written(&self, name: &str) -> WrittenPlace {
        match &self.words {
            Words::Independent => WrittenPlace::Untouched,
            Words::Ordered(evaluated) => evaluated
                .borrow()
                .as_ref()
                .map_or(WrittenPlace::Untouched, |words| words.state.written(name)),
        }
    }

    /// The existence fact the writes made ahead of the invocation — its own
    /// words' and then the enclosing evaluations' — leave `name` in, when
    /// they reached it: a write binds it, and any other touch (an unbind, an
    /// element of its array) states nothing.
    fn written_existence(&self, name: &str) -> Option<FactView> {
        let written = match self.words_written(name) {
            WrittenPlace::Untouched => written_in(&self.prior_writes, name),
            later => later,
        };
        match written {
            WrittenPlace::Untouched => None,
            WrittenPlace::Exact(_) => Some(FactView::Domain(DomainFact::Existence(
                Existence::Bound(BindingKind::Scalar),
            ))),
            WrittenPlace::Unknown => Some(FactView::Top(DeclineReason::StatefulNested)),
        }
    }

    /// `answer` with the writes the words made ahead of each outcome's own,
    /// for a route that does not carry them itself: the expression engine
    /// starts from the words' state, and every other route's outcome says
    /// nothing of what a word's command wrote.
    fn carrying_word_writes(&self, answer: LiftedAnswer, route: EvalRoute) -> LiftedAnswer {
        let Words::Ordered(evaluated) = &self.words else {
            return answer;
        };
        if matches!(route, EvalRoute::Expression { .. }) {
            return answer;
        }
        let Some(state) = evaluated.borrow().as_ref().map(|words| words.state.clone()) else {
            return answer;
        };
        let LiftedAnswer::Evaluated(mut outcomes) = answer else {
            return answer;
        };
        for outcome in &mut outcomes {
            let mut writes = state.writes.clone();
            writes.append(&mut outcome.nested_writes);
            outcome.nested_writes = writes;
            // The prefix an error completion ran counts the writes ahead of
            // it.
            if let CompletionOutcome::Error { written, .. } = &mut outcome.completion {
                *written += state.writes.len();
            }
            for binding in &state.evidence.bindings {
                record_binding(&mut outcome.evidence, binding.clone());
            }
        }
        LiftedAnswer::Evaluated(outcomes)
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
                Some(OperandSource::Substituted) => match &self.words {
                    Words::Independent => self.substituted(operand.text),
                    Words::Ordered(evaluated) => self.ordered_word(evaluated, id),
                },
                _ => simple_var_ref_name(operand.text, self.driver.lexer_config.braced_var)
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
        match domain {
            FactDomain::ExactValue => self.named_fact(name),
            FactDomain::Existence => self
                .written_existence(name)
                .unwrap_or_else(|| self.driver.existence_fact(self.ssa, name)),
            _ => FactView::Top(DeclineReason::Unavailable(self.driver.context.tier)),
        }
    }

    fn prior_store(&self, place: &PlaceRef, domain: FactDomain) -> FactView {
        match domain {
            FactDomain::ExactValue => {
                let written = match self.words_written(&place.name) {
                    WrittenPlace::Untouched => written_in(&self.prior_writes, &place.name),
                    later => later,
                };
                match written {
                    WrittenPlace::Exact(value) => return FactView::Exact(value, None),
                    WrittenPlace::Unknown => return FactView::Top(DeclineReason::NotExact),
                    WrittenPlace::Untouched => {}
                }
            }
            FactDomain::Existence => {
                return self
                    .written_existence(&place.name)
                    .unwrap_or_else(|| self.driver.existence_fact(self.ssa, &place.name));
            }
            _ => return FactView::Top(DeclineReason::Unavailable(self.driver.context.tier)),
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
                quoted: false,
                parts: whole(1),
            }),
            Some(OperandSource::Literal) => Ok(WordStructure {
                braced: false,
                quoted: false,
                parts: whole(0),
            }),
            // The content starts one past the opening quote.
            Some(OperandSource::QuotedLiteral) => Ok(WordStructure {
                braced: false,
                quoted: true,
                parts: whole(1),
            }),
            // A substituted word's reading is its parts'; its quoting is
            // not recorded.
            Some(OperandSource::Substituted) => Ok(WordStructure {
                braced: false,
                quoted: false,
                parts: word_parts(operand.text, self.driver.lexer_config)?,
            }),
            Some(OperandSource::Unknown) | None => Err(DeclineReason::NotExact),
        }
    }

    fn body(&self, id: OperandId) -> Result<BodyRegion, DeclineReason> {
        let operand = self.view.operand(id).ok_or(DeclineReason::NotExact)?;
        // Only text the word is: a brace-quoted script is not re-read as a
        // substitution, and any other spelling is not the script it runs.
        match self.sources.get(id.0) {
            Some(OperandSource::BracedLiteral) => Ok(BodyRegion {
                script: operand.text.to_owned(),
                base_offset: 0,
                frame: FrameLevel::Relative(0),
            }),
            _ => Err(DeclineReason::NotExact),
        }
    }

    fn nested(&self, script: &str, state: &mut EvaluationState) -> EvalAnswer {
        self.driver.nested_answer(script, state, self)
    }

    fn word_state(&self) -> Option<EvaluationState> {
        match &self.words {
            Words::Independent => None,
            Words::Ordered(evaluated) => {
                evaluated.borrow().as_ref().map(|words| words.state.clone())
            }
        }
    }

    fn math_function(&self, name: &str) -> Result<BindingIdentity, DeclineReason> {
        self.driver.math_function(name)
    }

    fn parameter_default(&self, procedure: &str, parameter: &str) -> ParameterDefault {
        self.driver.parameter_default(procedure, parameter)
    }

    fn context(&self) -> &AnalysisContext {
        &self.driver.context
    }
}

impl<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher> LatticeInputs<'_, S1, S2> {
    /// The fact operand `id` holds once every substituting word has been
    /// evaluated in order under one state ([`Words::Ordered`]): the first
    /// read evaluates them all, so the order of the reads never orders the
    /// writes.
    fn ordered_word(
        &self,
        evaluated: &RefCell<Option<Box<OrderedWords>>>,
        id: OperandId,
    ) -> FactView {
        let mut slot = evaluated.borrow_mut();
        let words = slot.get_or_insert_with(|| self.evaluate_words());
        words
            .facts
            .get(id.0)
            .and_then(Clone::clone)
            .unwrap_or(FactView::Top(DeclineReason::NotExact))
    }

    /// Every operand that substitutes, in operand order, under one
    /// `LocalWrites` state: Tcl substitutes a command's words left to right
    /// before it runs.
    fn evaluate_words(&self) -> Box<OrderedWords> {
        let mut state = EvaluationState::new(NestedPolicy::LocalWrites);
        let facts = self
            .view
            .operands
            .iter()
            .enumerate()
            .map(|(index, operand)| {
                (operand.kind == InvocationWordKind::Dynamic
                    && matches!(self.sources.get(index), Some(OperandSource::Substituted)))
                .then(|| self.substituted_in(operand.text, &mut state))
            })
            .collect();
        Box::new(OrderedWords { facts, state })
    }

    /// What `state`'s writes and then the lattice say `name` holds.
    fn read(&self, state: &EvaluationState, name: &str) -> FactView {
        match state.written(name) {
            WrittenPlace::Exact(value) => FactView::Exact(value, None),
            WrittenPlace::Unknown => FactView::Top(DeclineReason::NotExact),
            WrittenPlace::Untouched => self.named_fact(name),
        }
    }

    /// A substituted word's value: its parts' values concatenated — the
    /// literal runs decoded under the document's grammar, each variable
    /// read at this statement's use version, each script through the
    /// nested service under the effect-free policy. A lone variable read
    /// keeps the variable's own fact, identity included, so the lift can
    /// pin it. A part that is never exact makes the word never exact; else
    /// a pending part makes it pending.
    fn substituted(&self, text: &str) -> FactView {
        let mut state = EvaluationState::new(NestedPolicy::EffectFreeOnly);
        self.substituted_in(text, &mut state)
    }

    /// [`Self::substituted`] under `state`: a variable reads what the state's
    /// writes leave it, and a script's writes join the state in order, so a
    /// later part sees them.
    fn substituted_in(&self, text: &str, state: &mut EvaluationState) -> FactView {
        use tcl_lexer::word_parts::{SubstFlags, WordBody, WordPart as Part, decompose};
        if let Some(name) = simple_var_ref_name(text, self.driver.lexer_config.braced_var) {
            return self.read(state, name);
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
                    Ok(name) => self.read(state, &name).exact(),
                    Err(reason) => return FactView::Top(reason),
                },
                Part::Command(script) => match std::str::from_utf8(script) {
                    Ok(script) => match self.nested(script, state) {
                        // A command that did not complete normally ends the
                        // word with the writes so far: it has no value, and
                        // an error is one the invocation never gets past.
                        EvalAnswer::Evaluated(outcome)
                            if outcome.completion != CompletionOutcome::Normal =>
                        {
                            if matches!(outcome.completion, CompletionOutcome::Error { .. }) {
                                self.driver.word_error.set(true);
                            }
                            Err(EvalAnswer::Declined(DeclineReason::StatefulNested))
                        }
                        EvalAnswer::Evaluated(outcome) => match outcome.result {
                            ExactValueOrUnavailable::Exact(value) => Ok(value),
                            ExactValueOrUnavailable::Unavailable(_) => {
                                Err(EvalAnswer::Declined(DeclineReason::NotExact))
                            }
                        },
                        answer => Err(answer),
                    },
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

/// Whether a typed `Incr` over an absent place completes under every
/// release `registry`'s profile names — the release rule's
/// `creates_absent`, read from the declaration the typed node lowers from:
/// 8.5 onwards create the cell, 8.4 raises `can't read`. `false` when the
/// declaration says nothing.
pub(crate) fn typed_incr_creates_absent(registry: &CommandRegistry) -> bool {
    typed_node_commands(registry, LoweringHookId::Incr)
        .first()
        .and_then(|command| registry.get(command))
        .is_some_and(|spec| {
            match tcl_registry::value_transfer::resolve_semantics(spec, None, None) {
                tcl_registry::value_transfer::ResolvedSemantics::Derived(
                    tcl_registry::value_transfer::DerivedSemantics::CellUpdate(update),
                ) => update.creates_absent_under(
                    &tcl_registry::value_transfer::TargetSemantics::of(registry.profile()),
                ),
                _ => false,
            }
        })
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
    let inputs = StructureInputs::new(
        view_of(&resolved, &texts, &words, InvocationLayout::Source),
        &context,
    );
    match semantics.structure(&inputs) {
        PlanAnswer::CellReadModifyWrite {
            target, operation, ..
        } => Some((operation, target.0)),
        _ => None,
    }
}

/// Whether the plan a `head args…` call declares under `registry` absorbs
/// every completion of its body (`catch`'s), so a `return` the body runs
/// ends there: what the return reading asks of a call holding a script
/// word, answered from the registry's declaration rather than a command's
/// spelling.
pub(crate) fn resolved_body_absorbs_completion(
    registry: &CommandRegistry,
    head: &str,
    args: &[String],
) -> bool {
    let texts: Vec<&str> = args.iter().map(String::as_str).collect();
    let words: Vec<InvocationWord<'_>> = texts.iter().copied().map(word_of).collect();
    let Some(resolved) = registry
        .resolve_structured_invocation(
            InvocationWords::structured(InvocationWord::Literal(head), &words),
            registry.own_surface_query(),
        )
        .resolved()
    else {
        return false;
    };
    let Some(semantics) = resolved.semantics.value.semantics() else {
        return false;
    };
    let context = AnalysisContext::detached(registry.profile());
    let inputs = StructureInputs::new(
        view_of(&resolved, &texts, &words, InvocationLayout::Source),
        &context,
    );
    matches!(
        semantics.structure(&inputs),
        PlanAnswer::Body {
            completion: tcl_registry::value_transfer::CompletionProtocol::CatchAll { .. },
            ..
        }
    )
}

/// The declaration a `head args…` call over literal words resolves to under
/// `registry`, its form selected by those words, with the role the resolver
/// gives each word: what a consumer runs over the call's own words, or asks
/// whether the call writes its value word
/// ([`tcl_registry::value_transfer::ResolvedSemantics::writes_value_word`]).
pub(crate) fn resolved_literal_semantics(
    registry: &CommandRegistry,
    head: &str,
    args: &[&str],
) -> Option<(
    tcl_registry::value_transfer::ResolvedSemantics,
    Vec<(usize, ArgRole)>,
)> {
    let words: Vec<InvocationWord<'_>> =
        args.iter().copied().map(InvocationWord::Literal).collect();
    let resolved = registry
        .resolve_structured_invocation(
            InvocationWords::structured(InvocationWord::Literal(head), &words),
            registry.own_surface_query(),
        )
        .resolved()?;
    let roles = view_of(&resolved, args, &words, InvocationLayout::Source)
        .operands
        .iter()
        .enumerate()
        .filter_map(|(index, operand)| operand.role.map(|role| (index, role)))
        .collect();
    Some((resolved.semantics.value, roles))
}

/// The iteration plan a `head args…` call declares in its source layout
/// under `registry`, when it declares one: which word is a counted loop's
/// start script, condition and step script, or a conditional loop's
/// condition, and which is the body. The question the loop checks ask —
/// W240 to W242's counter and IRULE5003's bound — answered from the
/// registry's declaration rather than a command's spelling.
pub(crate) fn resolved_iteration_plan(
    registry: &CommandRegistry,
    head: &str,
    args: &[String],
) -> Option<IterationPlan> {
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
    let inputs = StructureInputs::new(
        view_of(&resolved, &texts, &words, InvocationLayout::Source),
        &context,
    );
    match semantics.structure(&inputs) {
        PlanAnswer::Iterate(plan) => Some(plan),
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

/// What one invocation does when a loop enumeration runs it over its state
/// ([`crate::static_loops`]): how it completes, and the stores it makes in
/// execution order, each resolved to its place.
#[derive(Debug, Clone)]
pub(crate) struct StateStep {
    /// How the invocation completes.
    pub(crate) completion: CompletionOutcome,
    /// The stores it makes, in order.
    pub(crate) stores: Vec<(PlaceRef, StoreOutcome)>,
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

impl AnalysisInputs for StateInputs<'_> {
    fn invocation(&self) -> &ResolvedInvocationView<'_> {
        &self.view
    }

    fn operand(&self, id: OperandId, domain: FactDomain) -> FactView {
        let Some(operand) = self.view.operand(id) else {
            return FactView::Top(DeclineReason::NotExact);
        };
        if operand.kind == InvocationWordKind::Literal {
            return match domain {
                FactDomain::ExactValue => {
                    FactView::Exact(ExactValue::from_literal(operand.text), None)
                }
                _ => FactView::Top(DeclineReason::Unavailable(AnalysisTier::Fast)),
            };
        }
        match simple_var_ref_name(operand.text, self.driver.context.grammar.braced_var) {
            Some(name) => self.variable(name, domain),
            None if domain == FactDomain::ExactValue => self
                .substituted(operand.text)
                .map_or_else(FactView::Top, |value| FactView::Exact(value, None)),
            None => FactView::Top(DeclineReason::NotExact),
        }
    }

    fn place(&self, id: OperandId) -> Result<PlaceRef, DeclineReason> {
        let operand = self.view.operand(id).ok_or(DeclineReason::NotExact)?;
        if operand.kind != InvocationWordKind::Literal {
            return Err(DeclineReason::DynamicName);
        }
        // The word names the place as written: `a(k)` is an element of `a`,
        // never the scalar `a`.
        Ok(place_named(operand.text))
    }

    fn variable(&self, name: &str, domain: FactDomain) -> FactView {
        use crate::static_loops::Slot;
        match (self.state.get(name), domain) {
            (Some(Slot::Value(value)), FactDomain::ExactValue) => {
                FactView::Exact(value.clone(), None)
            }
            (Some(Slot::Value(_) | Slot::Scalar), FactDomain::Existence) => {
                FactView::Domain(DomainFact::Existence(Existence::Bound(BindingKind::Scalar)))
            }
            (Some(Slot::Scalar), FactDomain::ExactValue) => FactView::Top(DeclineReason::NotExact),
            (Some(Slot::Unbound), FactDomain::Existence) => {
                FactView::Domain(DomainFact::Existence(Existence::Unbound))
            }
            (Some(Slot::Unbound), FactDomain::ExactValue) => {
                FactView::Top(DeclineReason::UnboundPlace)
            }
            (None, FactDomain::ExactValue | FactDomain::Existence) => {
                FactView::Top(DeclineReason::NotExact)
            }
            _ => FactView::Top(DeclineReason::Unavailable(AnalysisTier::Fast)),
        }
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

    fn nested(&self, script: &str, state: &mut EvaluationState) -> EvalAnswer {
        if state.policy != NestedPolicy::EffectFreeOnly {
            return EvalAnswer::Declined(DeclineReason::StatefulNested);
        }
        self.run_nested(script)
    }

    fn math_function(&self, name: &str) -> Result<BindingIdentity, DeclineReason> {
        self.driver.math_function(name)
    }

    fn parameter_default(&self, procedure: &str, parameter: &str) -> ParameterDefault {
        self.driver.parameter_default(procedure, parameter)
    }

    fn context(&self) -> &AnalysisContext {
        &self.driver.context
    }
}

impl StateInputs<'_> {
    /// A substituted word's value over the state: its parts concatenated —
    /// each literal run decoded under the document's grammar, each variable
    /// read from the state, each script run as one command ([`Self::run_nested`]).
    /// Any part the state does not decide declines the word.
    fn substituted(&self, text: &str) -> Result<ExactValue, DeclineReason> {
        use tcl_lexer::word_parts::{SubstFlags, WordBody, WordPart as Part, decompose};
        let parts = match decompose(
            text.as_bytes(),
            SubstFlags::default(),
            self.driver.lexer_config,
        ) {
            WordBody::Literal(bytes) => return Ok(exact_of_bytes(bytes.to_vec())),
            WordBody::Parts(parts) => parts,
        };
        let mut bytes = Vec::with_capacity(text.len());
        for part in parts {
            let value = match part {
                Part::Text(run) => {
                    bytes.extend_from_slice(&run);
                    continue;
                }
                Part::Variable(reference) => {
                    let name = variable_name(&reference)?;
                    match self.variable(&name, FactDomain::ExactValue) {
                        FactView::Exact(value, _) => value,
                        FactView::Top(reason) => return Err(reason),
                        _ => return Err(DeclineReason::NotExact),
                    }
                }
                Part::Command(script) => {
                    let script = std::str::from_utf8(script).map_err(|_| DeclineReason::NotText)?;
                    match self.run_nested(script) {
                        EvalAnswer::Evaluated(outcome)
                            if outcome.completion == CompletionOutcome::Normal =>
                        {
                            match outcome.result {
                                ExactValueOrUnavailable::Exact(value) => value,
                                ExactValueOrUnavailable::Unavailable(_) => {
                                    return Err(DeclineReason::NotExact);
                                }
                            }
                        }
                        // A script that does not complete normally ends the
                        // word, which the enumeration does not follow.
                        EvalAnswer::Evaluated(_) => return Err(DeclineReason::StatefulNested),
                        EvalAnswer::Pending => return Err(DeclineReason::NotExact),
                        EvalAnswer::Declined(reason) => return Err(reason),
                    }
                }
                Part::ParseError(_) => return Err(DeclineReason::WrongRepresentation),
            };
            bytes.extend_from_slice(&value.bytes);
        }
        Ok(exact_of_bytes(bytes))
    }

    /// The one command of `script` run over the state by its declared route —
    /// a registry-owned evaluator or the expression engine — under the
    /// effect-free policy: a command that stores declines, as one with no
    /// route or a head the run may not take for the registry's does.
    fn run_nested(&self, script: &str) -> EvalAnswer {
        let commands = crate::segmenter::segment_commands_with_offset_and_config(
            script,
            0,
            self.driver.lexer_config,
        );
        let [seg] = commands.as_slice() else {
            return EvalAnswer::Declined(DeclineReason::Unsupported);
        };
        let head = seg.name();
        if split_head(script).0 != head {
            return EvalAnswer::Declined(DeclineReason::Unsupported);
        }
        if !self.driver.enumeration_trusts(head) {
            return EvalAnswer::Declined(DeclineReason::RebindingSuspected);
        }
        let cooked = self.driver.cooked_args(seg);
        let texts: Vec<&str> = cooked.iter().map(|arg| arg.text.as_ref()).collect();
        let words: Vec<InvocationWord<'_>> = cooked.iter().map(ArgWord::word).collect();
        let Some(resolved) = self.driver.resolve(head, &words) else {
            return EvalAnswer::Declined(DeclineReason::NoSemantics);
        };
        let Some(semantics) = resolved.semantics.value.semantics() else {
            return EvalAnswer::Declined(DeclineReason::NoSemantics);
        };
        let inputs = StateInputs {
            driver: self.driver,
            view: view_of(&resolved, &texts, &words, InvocationLayout::Source),
            state: self.state,
        };
        let mut budget = self.driver.budget();
        let answer = match semantics.route() {
            EvalRoute::Direct { id } if id.owner() == EvaluatorOwner::Registry => {
                semantics.evaluate(&inputs, &mut budget)
            }
            // A pack's arity or option row can switch the declared route
            // off, as the lattice driver asks before it runs the engine.
            EvalRoute::Expression { language } => match semantics
                .as_declared()
                .and_then(|declared| declared.invocation_decline(&inputs))
            {
                Some(EvalAnswer::Pending) => EvalAnswer::Pending,
                Some(EvalAnswer::Declined(reason)) => EvalAnswer::Declined(reason),
                Some(EvalAnswer::Evaluated(_)) | None => ExpressionEvaluation {
                    expression: Expression::Assembled(ExpressionRoute { language }),
                    policy: self.driver.policy,
                    nested: NestedPolicy::EffectFreeOnly,
                    head: Some(binding_of(head, resolved.canonical_command)),
                }
                .evaluate(&inputs, &mut budget),
            },
            EvalRoute::None { reason } => EvalAnswer::Declined(DeclineReason::NoRoute(reason)),
            _ => EvalAnswer::Declined(DeclineReason::Unsupported),
        };
        match answer {
            EvalAnswer::Evaluated(outcome)
                if !outcome.ordered_stores.is_empty() || !outcome.nested_writes.is_empty() =>
            {
                EvalAnswer::Declined(DeclineReason::StatefulNested)
            }
            answer => answer,
        }
    }
}

impl LatticeDriver<'_> {
    /// Whether a loop enumeration may take `head` for the registry's command:
    /// under the run's trust stance when it has one, and the registry's
    /// table where it has none, as a math function's binding is read
    /// ([`Self::math_function`]).
    fn enumeration_trusts(&self, head: &str) -> bool {
        self.folds.is_none() || self.trusted(head)
    }

    /// The commands the typed statement of `hook` stands for, when a loop
    /// enumeration may take every one for the registry's: the head it is
    /// resolved through first.
    pub(crate) fn enumeration_typed_head(&self, hook: LoweringHookId) -> Option<&str> {
        let heads = typed_node_commands(self.registry, hook);
        (heads.iter().all(|head| self.enumeration_trusts(head)))
            .then(|| heads.first().copied())
            .flatten()
    }

    /// `head` with its source words, each with whether it was braced, run
    /// over a loop enumeration's `state` through the registry's own route
    /// ([`StateStep`]). `Err` with the reason when the head may not be the
    /// registry's command, declares no registry-owned route, or the route
    /// declines over the state.
    pub(crate) fn invoke_in_state(
        &self,
        state: &crate::static_loops::LoopState,
        head: &str,
        words: &[(&str, bool)],
        budget: &mut Budget,
    ) -> Result<StateStep, DeclineReason> {
        if !self.enumeration_trusts(head) {
            return Err(DeclineReason::RebindingSuspected);
        }
        let texts: Vec<&str> = words.iter().map(|&(text, _)| text).collect();
        let classified: Vec<InvocationWord<'_>> = words
            .iter()
            .map(|&(text, braced)| amount_word((text, braced)))
            .collect();
        let resolved = self
            .resolve(head, &classified)
            .ok_or(DeclineReason::NoSemantics)?;
        let semantics = resolved
            .semantics
            .value
            .semantics()
            .ok_or(DeclineReason::NoSemantics)?;
        match semantics.route() {
            EvalRoute::Direct { id } if id.owner() == EvaluatorOwner::Registry => {}
            EvalRoute::None { reason } => return Err(DeclineReason::NoRoute(reason)),
            _ => return Err(DeclineReason::Unsupported),
        }
        let inputs = StateInputs {
            driver: self,
            view: view_of(&resolved, &texts, &classified, InvocationLayout::Source),
            state,
        };
        let outcome = match semantics.evaluate(&inputs, budget) {
            EvalAnswer::Evaluated(outcome) => outcome,
            EvalAnswer::Pending => return Err(DeclineReason::NotExact),
            EvalAnswer::Declined(reason) => return Err(reason),
        };
        let mut stores = outcome.nested_writes.clone();
        for store in &outcome.ordered_stores {
            stores.push((inputs.place(store.target().0)?, store.clone()));
        }
        Ok(StateStep {
            completion: outcome.completion.clone(),
            stores,
        })
    }

    /// `node` evaluated by the expression route over a loop enumeration's
    /// `state`: its value, how it completes when it does not complete
    /// normally, or why it declines. `head` names the command the
    /// expression's statement fused, when one did, which the run's trust
    /// stance must take for the registry's.
    pub(crate) fn expression_in_state(
        &self,
        state: &crate::static_loops::LoopState,
        node: &ExprNode,
        head: Option<(&str, &str)>,
        budget: &mut Budget,
    ) -> Result<Result<ExactValue, CompletionOutcome>, DeclineReason> {
        if let Some((head, _)) = head
            && !self.enumeration_trusts(head)
        {
            return Err(DeclineReason::RebindingSuspected);
        }
        let expression = ExpressionEvaluation {
            expression: Expression::Parsed(node),
            policy: self.policy,
            nested: NestedPolicy::EffectFreeOnly,
            head: head.map(|(head, identity)| binding_of(head, identity)),
        };
        let inputs = StateInputs {
            driver: self,
            view: ResolvedInvocationView {
                canonical_command: "expr",
                subcommand: None,
                form: None,
                layout: InvocationLayout::Source,
                operands: Vec::new(),
                argument_offset: 0,
                arity: None,
            },
            state,
        };
        match expression.evaluate(&inputs, budget) {
            EvalAnswer::Evaluated(outcome) => match (outcome.completion, outcome.result) {
                (CompletionOutcome::Normal, ExactValueOrUnavailable::Exact(value)) => Ok(Ok(value)),
                (CompletionOutcome::Normal, ExactValueOrUnavailable::Unavailable(_)) => {
                    Err(DeclineReason::NotExact)
                }
                (completion, _) => Ok(Err(completion)),
            },
            EvalAnswer::Pending => Err(DeclineReason::NotExact),
            EvalAnswer::Declined(reason) => Err(reason),
        }
    }

    /// The iteration plan `head` with its source words declares, its operands
    /// read over a loop enumeration's `state`.
    pub(crate) fn loop_plan_in_state(
        &self,
        state: &crate::static_loops::LoopState,
        head: &str,
        words: &[(&str, bool)],
    ) -> Result<IterationPlan, DeclineReason> {
        if !self.enumeration_trusts(head) {
            return Err(DeclineReason::RebindingSuspected);
        }
        let texts: Vec<&str> = words.iter().map(|&(text, _)| text).collect();
        let classified: Vec<InvocationWord<'_>> = words
            .iter()
            .map(|&(text, braced)| amount_word((text, braced)))
            .collect();
        let resolved = self
            .resolve(head, &classified)
            .ok_or(DeclineReason::NoSemantics)?;
        let semantics = resolved
            .semantics
            .value
            .semantics()
            .ok_or(DeclineReason::NoSemantics)?;
        let inputs = StateInputs {
            driver: self,
            view: view_of(&resolved, &texts, &classified, InvocationLayout::Source),
            state,
        };
        match semantics.structure(&inputs) {
            PlanAnswer::Iterate(plan) => Ok(plan),
            PlanAnswer::Declined(reason) => Err(reason),
            _ => Err(DeclineReason::Unsupported),
        }
    }

    /// A source word's value over a loop enumeration's `state`: a braced
    /// word's text, a bare word with nothing to substitute cooked as Tcl
    /// reads it, a whole-variable read of the state, or a word that
    /// substitutes, its parts read over the state and its scripts run as one
    /// command each ([`StateInputs::substituted`]).
    pub(crate) fn word_in_state(
        &self,
        state: &crate::static_loops::LoopState,
        text: &str,
        braced: bool,
    ) -> Result<ExactValue, DeclineReason> {
        use crate::static_loops::Slot;
        if braced {
            return self
                .literal_value(text, TokenType::Str)
                .map(|value| ExactValue::from_literal(&value))
                .ok_or(DeclineReason::NotExact);
        }
        if let Some(name) = simple_var_ref_name(text, self.context.grammar.braced_var) {
            // `${a(k)}` reads the element, never the scalar `a`.
            return match state.get(name) {
                Some(Slot::Value(value)) => Ok(value.clone()),
                Some(Slot::Unbound) => Err(DeclineReason::UnboundPlace),
                Some(Slot::Scalar) | None => Err(DeclineReason::NotExact),
            };
        }
        if text.contains(['$', '[']) {
            let inputs = StateInputs {
                driver: self,
                view: ResolvedInvocationView {
                    canonical_command: "",
                    subcommand: None,
                    form: None,
                    layout: InvocationLayout::Source,
                    operands: Vec::new(),
                    argument_offset: 0,
                    arity: None,
                },
                state,
            };
            return inputs.substituted(text);
        }
        self.literal_value(text, TokenType::Esc)
            .map(|value| ExactValue::from_literal(&value))
            .ok_or(DeclineReason::NotExact)
    }

    /// The elements of a list value under the target's list rules, or `None`
    /// for a text that is no list, on which a command reading it raises.
    pub(crate) fn list_elements(&self, text: &str) -> Option<Vec<String>> {
        self.policy
            .word_rules
            .split_list(text)
            .ok()
            .map(|elements| elements.into_iter().map(Cow::into_owned).collect())
    }

    /// The selection the lowered case-list statement `stmt` makes when its
    /// subject holds `subject` ([`statement_selection`]).
    pub(crate) fn switch_selection(
        &self,
        stmt: &Statement,
        subject: &str,
    ) -> Option<SelectionFact> {
        statement_selection(self.registry, stmt, subject)
    }

    /// The budget a loop enumeration charges every statement of every
    /// iteration to, within the run's request.
    pub(crate) fn enumeration_budget(&self) -> Budget {
        self.budget()
    }

    /// A condition's truth over a loop enumeration's `state`, as `if` and a
    /// loop read it ([`Self::expression_in_state`]): `Err` when it declines or
    /// does not complete normally, or when its value is no boolean.
    pub(crate) fn condition_in_state(
        &self,
        state: &crate::static_loops::LoopState,
        node: &ExprNode,
        budget: &mut Budget,
    ) -> Result<bool, DeclineReason> {
        match self.expression_in_state(state, node, None, budget)? {
            Ok(value) => truth_of(&ExactValueOrUnavailable::Exact(value))
                .ok_or(DeclineReason::WrongRepresentation),
            Err(_) => Err(DeclineReason::Unsupported),
        }
    }
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

/// The existence state a statement's scripts start from, set on the driver
/// while this lives ([`LatticeDriver::body_entry_scope`]).
pub(crate) struct BodyEntryScope<'d> {
    entry: &'d RefCell<Option<Vec<Existence>>>,
}

impl Drop for BodyEntryScope<'_> {
    fn drop(&mut self) {
        *self.entry.borrow_mut() = None;
    }
}

/// What a variable holds, as a store into it needs to know.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Held {
    /// No binding.
    Nothing,
    /// A scalar.
    Scalar,
    /// An array.
    Array,
}

/// Whether each store a protected script's state took from `from_index` on is
/// one its place certainly takes, given the state's writes before it and the
/// fact before the script ([`held_before`]): a scalar store into a variable
/// that holds nothing or a scalar, an element store into one that holds
/// nothing or an array. Tcl raises on any other, so where the analysis does
/// not prove the variable's kind the store may fail and the script's
/// completion is not exact.
fn proved_stores<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
    writes: &[(PlaceRef, StoreOutcome)],
    from_index: usize,
    from: &LatticeInputs<'_, S1, S2>,
) -> Result<(), EvalAnswer> {
    for at in from_index..writes.len() {
        let (place, store) = &writes[at];
        if matches!(store, StoreOutcome::Preserve { .. }) {
            continue;
        }
        let held = held_before(place.base(), &writes[..at], from)?;
        let takes = if place.is_element() {
            held != Held::Scalar
        } else {
            held != Held::Array
        };
        if !takes {
            return Err(EvalAnswer::Declined(DeclineReason::NotExact));
        }
    }
    Ok(())
}

/// What the variable `name` holds after `earlier`, a protected script's writes
/// so far: the last of them that reaches it decides — a scalar write a scalar,
/// an element write an array — and with none, the fact before the script.
/// A write that may have happened, and a kind no fact proves, decline.
fn held_before<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
    name: &str,
    earlier: &[(PlaceRef, StoreOutcome)],
    from: &LatticeInputs<'_, S1, S2>,
) -> Result<Held, EvalAnswer> {
    let unproven = || Err(EvalAnswer::Declined(DeclineReason::NotExact));
    for (place, store) in earlier.iter().rev() {
        if place.base() != name || matches!(store, StoreOutcome::Preserve { .. }) {
            continue;
        }
        return match store {
            StoreOutcome::Unbind { .. } if !place.is_element() => Ok(Held::Nothing),
            StoreOutcome::Write { .. }
            | StoreOutcome::WriteElement { .. }
            | StoreOutcome::WriteUnavailable { .. } => Ok(if place.is_element() {
                Held::Array
            } else {
                Held::Scalar
            }),
            _ => unproven(),
        };
    }
    match from.prior_store(&PlaceRef::scalar(name), FactDomain::Existence) {
        FactView::Domain(DomainFact::Existence(Existence::Unbound)) => Ok(Held::Nothing),
        FactView::Domain(DomainFact::Existence(Existence::Bound(BindingKind::Scalar))) => {
            Ok(Held::Scalar)
        }
        FactView::Domain(DomainFact::Existence(Existence::Bound(BindingKind::Array))) => {
            Ok(Held::Array)
        }
        FactView::Pending | FactView::Domain(DomainFact::Existence(Existence::Pending)) => {
            Err(EvalAnswer::Pending)
        }
        _ => unproven(),
    }
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

/// The name a `$var` / `${var}` word reads, or `None` for any other shape:
/// a bare `$name` of name characters, or one `${…}` reference whose closer
/// — located by the variable-name owner under `style`, the document's
/// release rule — is the word's last byte.
fn simple_var_ref_name(text: &str, style: tcl_dialect::BracedVarStyle) -> Option<&str> {
    if text.starts_with("${") {
        // `${a} + ${b}` starts and ends like one braced reference and is not.
        return crate::naming::split_braced_var_ref(text, style)
            .and_then(|(name, rest)| rest.is_empty().then_some(name));
    }
    let name = text.strip_prefix('$')?;
    name.bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b':')
        .then_some(name)
}

/// The variable a branch condition's `Raw` operand reads (§ `switch`, step
/// 1): the variable-name owner proves the text is exactly one reference
/// under `style` ([`simple_var_ref_name`]), and the name holds none of `{`,
/// `}` or `\`, the three characters on which the release close rules and
/// the name readers can disagree — so a backslash-bearing `${…}` subject,
/// which `Raw` exists to carry intact under either rule, stays `Raw` and
/// decides nothing. Any other `Raw` text is never read as a variable.
pub(crate) fn whole_variable_operand(
    text: &str,
    style: tcl_dialect::BracedVarStyle,
) -> Option<&str> {
    simple_var_ref_name(text, style).filter(|name| !name.contains(['{', '}', '\\']))
}

/// Inputs for a structure-only question over source words: literal words
/// are exact, dynamic words are not, and a literal name is a place.
struct StructureInputs<'a> {
    view: ResolvedInvocationView<'a>,
    context: &'a AnalysisContext,
    /// What a prior store reads.
    prior: Prior<'a>,
    /// The operands the plan has read as exact values, in the order it read
    /// them.
    exact_reads: RefCell<Vec<OperandId>>,
}

/// What a structure question reads as the prior value of a place.
#[derive(Clone, Copy)]
enum Prior<'a> {
    /// Nothing: the question has no store to read.
    Unavailable,
    /// The value the caller knows, for the one place a plan reads (a body
    /// plan's dictionary).
    Known(&'a str),
    /// The least dictionary holding, each inside the one before, the exact
    /// words the plan read before it: any key path the plan descends
    /// exists and holds no key, so the plan answers its shape and binds
    /// nothing.
    KeyPath,
}

impl<'a> StructureInputs<'a> {
    fn new(view: ResolvedInvocationView<'a>, context: &'a AnalysisContext) -> Self {
        Self {
            view,
            context,
            prior: Prior::Unavailable,
            exact_reads: RefCell::default(),
        }
    }

    /// The texts of the operands the plan has read as exact values, in the
    /// order it read them.
    fn exact_read_texts(&self) -> Vec<&'a str> {
        self.exact_reads
            .borrow()
            .iter()
            .filter_map(|&id| self.view.operand(id).map(|operand| operand.text))
            .collect()
    }
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
                self.exact_reads.borrow_mut().push(id);
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

    fn prior_store(&self, _place: &PlaceRef, domain: FactDomain) -> FactView {
        match self.prior {
            Prior::Known(prior) if domain == FactDomain::ExactValue => {
                FactView::Exact(ExactValue::from_literal(prior), None)
            }
            Prior::KeyPath if domain == FactDomain::ExactValue => {
                let dictionary = self
                    .exact_read_texts()
                    .into_iter()
                    .rev()
                    .fold(String::new(), |inner, key| {
                        tcl_syntax::list::join_list([key, inner.as_str()])
                    });
                FactView::Exact(ExactValue::from_literal(&dictionary), None)
            }
            _ => FactView::Top(DeclineReason::Unavailable(AnalysisTier::Structure)),
        }
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

/// One variable a dictionary body binds on entry ([`dict_body`]).
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum DictBinder {
    /// A key the dictionary holds, bound to the variable of its name, with
    /// the value it binds.
    Key {
        /// The key and variable name.
        name: String,
        /// The key's value.
        value: String,
    },
    /// The variable operand at `variable`, bound to the value of the key
    /// operand before it when the dictionary holds that key.
    Variable {
        /// The variable operand's argument index.
        variable: usize,
        /// Whether the dictionary holds the key, so the variable is bound on
        /// entry.
        bound: bool,
    },
}

/// The dictionary operand, the binders and the key path of the plan `head
/// args…` declares, when it binds a dictionary's keys into its body and
/// writes them back (`dict with`, `dict update`, under whichever spelling
/// the registry resolves), asked over `prior`. The key path is the exact
/// words the plan read on its way to the dictionary's keys.
fn dict_body_plan(
    registry: &CommandRegistry,
    head: &str,
    args: &[String],
    prior: Prior<'_>,
) -> Option<(
    OperandId,
    Vec<tcl_registry::value_transfer::Binder>,
    Vec<String>,
)> {
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
    let mut inputs = StructureInputs::new(
        view_of(&resolved, &texts, &words, InvocationLayout::Source),
        &context,
    );
    inputs.prior = prior;
    let PlanAnswer::Body {
        binders,
        reconcile: tcl_registry::value_transfer::Reconcile::WriteBackKeys(dict),
        ..
    } = semantics.structure(&inputs)
    else {
        return None;
    };
    let path = inputs
        .exact_read_texts()
        .into_iter()
        .map(str::to_owned)
        .collect();
    Some((dict, binders, path))
}

/// The argument index of the dictionary variable when the plan `head
/// args…` declares binds a dictionary's keys into its body and writes them
/// back — whatever the dictionary holds, so a call whose keys the analysis
/// cannot name is still found.
pub(crate) fn dict_body_operand(
    registry: &CommandRegistry,
    head: &str,
    args: &[String],
) -> Option<usize> {
    dict_body_plan(registry, head, args, Prior::KeyPath).map(|(dict, ..)| dict.0)
}

/// What the dictionary body `head args…` binds on entry when its
/// dictionary holds `dictionary`: each key the plan declares, with its
/// value at the plan's key path, and each variable operand, with whether
/// the dictionary holds its key. `None` when the call is no dictionary
/// body, or its dictionary does not hold the key path (the command's
/// error).
pub(crate) fn dict_body(
    registry: &CommandRegistry,
    head: &str,
    args: &[String],
    dictionary: &str,
) -> Option<Vec<DictBinder>> {
    let (_, binders, path) = dict_body_plan(registry, head, args, Prior::Known(dictionary))?;
    let rules = WordValueRules::of_profile(registry.profile());
    let level = path.iter().try_fold(dictionary.to_owned(), |level, key| {
        dict_value_at(rules, &level, key)
    })?;
    Some(
        binders
            .into_iter()
            .filter_map(|binder| match binder.name {
                BinderName::Declared(name) => {
                    let value = dict_value_at(rules, &level, &name)?;
                    Some(DictBinder::Key { name, value })
                }
                // `dict update d k v …`: the variable takes the value of the
                // key word before it, which must be literal to be looked up.
                BinderName::Operand(id) => id.0.checked_sub(1).map(|key| DictBinder::Variable {
                    variable: id.0,
                    bound: args.get(key).is_some_and(|word| {
                        matches!(word_of(word), InvocationWord::Literal(_))
                            && dict_value_at(rules, &level, word).is_some()
                    }),
                }),
            })
            .collect(),
    )
}

/// The value `key` maps to in the dictionary `text`, split as the
/// dialect's `rules` split a list: the last pair's, as `dict get` reads a
/// list with a repeated key.
fn dict_value_at(rules: WordValueRules, text: &str, key: &str) -> Option<String> {
    let elements = rules.split_list(text).ok()?;
    if !elements.len().is_multiple_of(2) {
        return None;
    }
    elements
        .as_chunks::<2>()
        .0
        .iter()
        .rev()
        .find(|[held, _]| held.as_ref() == key)
        .map(|[_, value]| value.to_string())
}

/// The element writes the call `head args…` states over its literal words:
/// each `(array, key, value)` its registry route's outcome writes by key
/// (`array set arr {k v …}`). The lattice holds the same writes in a
/// function without a barrier, and loses them in one with a barrier, which
/// widens every value the function holds; a flow-insensitive reader takes
/// the statement's own. Only a call with a declared store target is run.
pub(crate) fn literal_element_writes(
    registry: &CommandRegistry,
    head: &str,
    args: &[String],
) -> Vec<(String, String, String)> {
    let texts: Vec<&str> = args.iter().map(String::as_str).collect();
    let words: Vec<InvocationWord<'_>> = texts.iter().copied().map(word_of).collect();
    let Some(resolved) = registry
        .resolve_structured_invocation(
            InvocationWords::structured(InvocationWord::Literal(head), &words),
            registry.own_surface_query(),
        )
        .resolved()
    else {
        return Vec::new();
    };
    let Some(semantics) = resolved.semantics.value.semantics() else {
        return Vec::new();
    };
    let context = AnalysisContext::detached(registry.profile());
    let inputs = StructureInputs::new(
        view_of(&resolved, &texts, &words, InvocationLayout::Source),
        &context,
    );
    if semantics.store_targets(&inputs).is_empty() {
        return Vec::new();
    }
    let EvalAnswer::Evaluated(outcome) = semantics.evaluate(&inputs, &mut Budget::evaluation())
    else {
        return Vec::new();
    };
    outcome
        .ordered_stores
        .into_iter()
        .filter_map(|store| match store {
            StoreOutcome::WriteElement { target, key, value } => {
                let array = inputs.place(target.0).ok()?.name;
                Some((array, key, String::from_utf8(value.bytes).ok()?))
            }
            _ => None,
        })
        .collect()
}

/// How one word of a call reads in its source, for a template plan asked
/// before the lattice ([`literal_template_plan`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceWord {
    /// A brace-quoted word: its content is its value and its text.
    Braced,
    /// A bare or quoted word the parser leaves as literal text.
    Literal,
    /// A word the parser substitutes: its value is computed.
    Substituted,
}

impl SourceWord {
    /// The word `text` reads as: brace-quoted when `braced`, else computed
    /// when it spells a `$` or `[`, else literal text (as when no text is
    /// given).
    #[must_use]
    pub fn of(text: Option<&str>, braced: bool) -> Self {
        if braced {
            Self::Braced
        } else if text.is_some_and(|text| text.contains('$') || text.contains('[')) {
            Self::Substituted
        } else {
            Self::Literal
        }
    }
}

/// The template-word plan the call `head args…` declares over its source
/// words, each read as `source` says — the plan a folder that runs before
/// the lattice, or the analyser's walk, reads
/// (`docs/design/compiler/value-transfers.md` § *The template-word plan*).
/// A literal word reads as its own spelling and a substituted one is
/// unproven, so a computed switch runs every kind. `None` when the call
/// declares no template plan or the plan declines.
#[must_use]
pub fn literal_template_plan(
    registry: &CommandRegistry,
    head: &str,
    args: &[&str],
    source: impl Fn(usize) -> SourceWord,
) -> Option<tcl_registry::value_transfer::TemplateWordPlan> {
    let words: Vec<InvocationWord<'_>> = args.iter().copied().map(word_of).collect();
    let resolved = registry
        .resolve_structured_invocation(
            InvocationWords::structured(InvocationWord::Literal(head), &words),
            registry.own_surface_query(),
        )
        .resolved()?;
    let semantics = resolved.semantics.value.semantics()?;
    let config = LexerConfig::for_profile(registry.profile());
    let mut inputs = tcl_registry::value_transfer::LiteralInputs::new(
        resolved.canonical_command,
        None,
        args,
        registry.profile(),
    );
    for (index, text) in args.iter().enumerate() {
        let id = OperandId(index);
        inputs = match source(index) {
            SourceWord::Braced => inputs.with_braced(id),
            SourceWord::Literal => inputs.with_structure(
                id,
                WordStructure {
                    braced: false,
                    quoted: false,
                    parts: vec![WordPart::Literal {
                        span: Span::new(0, u32::try_from(text.len()).unwrap_or(u32::MAX)),
                        text: (*text).to_owned(),
                    }],
                },
            ),
            SourceWord::Substituted => {
                let inputs = inputs.with_unproven(id);
                match word_parts(text, config) {
                    Ok(parts) => inputs.with_structure(
                        id,
                        WordStructure {
                            braced: false,
                            quoted: false,
                            parts,
                        },
                    ),
                    Err(_) => inputs,
                }
            }
        };
    }
    match semantics.structure(&inputs) {
        PlanAnswer::TemplateWord(plan) => Some(plan),
        _ => None,
    }
}

/// How one word of a case-list call was written, for a consumer that reads
/// the call's words rather than the lattice ([`literal_selection`]). The
/// releases part on a fall-through body's delimiters, so a word whose value
/// the caller proves says how it was spelled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WordForm {
    /// A bare word with nothing to substitute.
    Bare,
    /// A double-quoted word with nothing to substitute.
    Quoted,
    /// A brace-quoted word.
    Braced,
    /// A word whose value the caller does not prove.
    Unproven,
}

/// The selection a case-list call makes over words whose values the caller
/// proves, with what its indices read against.
#[derive(Debug, Clone)]
pub(crate) struct LiteralSelection {
    /// The selection: one entry, for the one subject value the words state.
    pub fact: SelectionFact,
    /// The plan's subject word, an index into the call's words.
    pub subject: usize,
    /// How many pattern and body pairs the plan read.
    pub clauses: usize,
}

/// The inputs over `words` for the resolved case-list command `command`:
/// every word exact but the unproven ones, each with the structure its form
/// states.
fn literal_selection_inputs<'a>(
    command: &'a str,
    words: &[(&'a str, WordForm)],
    profile: Option<&'static tcl_dialect::DialectProfile>,
) -> tcl_registry::value_transfer::LiteralInputs<'a> {
    let texts: Vec<&str> = words.iter().map(|&(text, _)| text).collect();
    let mut inputs =
        tcl_registry::value_transfer::LiteralInputs::new(command, None, &texts, profile);
    for (index, &(_, form)) in words.iter().enumerate() {
        let id = OperandId(index);
        inputs = match form {
            WordForm::Bare => inputs.with_bare(id),
            WordForm::Quoted => inputs.with_quoted(id),
            WordForm::Braced => inputs.with_braced(id),
            WordForm::Unproven => inputs.with_unproven(id),
        };
    }
    inputs
}

/// The selection the case-list call `head words…` makes when the caller
/// proves the words' values (`docs/design/compiler/value-transfers.md`
/// § *`switch`*): the registry's declared `Selection` transfer for the
/// resolved command, the shared core choosing the arm. `subject` gives the
/// plan's subject word the value the caller proves for it when the words
/// spell it otherwise (`$mode`). `None` where the head declares no
/// selection, or the plan or the transfer declines.
pub(crate) fn literal_selection(
    registry: &CommandRegistry,
    head: &str,
    words: &[(&str, WordForm)],
    subject: Option<&str>,
) -> Option<LiteralSelection> {
    let invocation: Vec<InvocationWord<'_>> = words
        .iter()
        .map(|&(text, form)| match form {
            WordForm::Unproven => InvocationWord::Dynamic,
            _ => InvocationWord::Literal(text),
        })
        .collect();
    let resolved = registry
        .resolve_structured_invocation(
            InvocationWords::structured(InvocationWord::Literal(head), &invocation),
            registry.own_surface_query(),
        )
        .resolved()?;
    let semantics = resolved.semantics.value.semantics()?;
    let command = resolved.canonical_command;
    let profile = registry.profile();
    let mut inputs = literal_selection_inputs(command, words, profile);
    let mut plan = semantics.structure(&inputs);
    if let (Some(value), PlanAnswer::CaseList { subject: at, .. }) = (subject, &plan) {
        let mut proven = words.to_vec();
        *proven.get_mut(at.0)? = (value, WordForm::Bare);
        inputs = literal_selection_inputs(command, &proven, profile);
        plan = semantics.structure(&inputs);
    }
    let PlanAnswer::CaseList {
        subject: at, arms, ..
    } = plan
    else {
        return None;
    };
    let clauses = plan_clause_count(
        &inputs,
        &arms,
        WordValueRules::from_config(&LexerConfig::for_profile(profile)),
    )?;
    let TransferAnswer::Selection(fact) =
        semantics.transfer(FactDomain::Selection, &inputs, &mut Budget::evaluation())
    else {
        return None;
    };
    Some(LiteralSelection {
        fact,
        subject: at.0,
        clauses,
    })
}

/// The form a lowered word's source says it has.
const fn form_of(source: OperandSource) -> WordForm {
    match source {
        OperandSource::Literal => WordForm::Bare,
        OperandSource::QuotedLiteral => WordForm::Quoted,
        OperandSource::BracedLiteral => WordForm::Braced,
        OperandSource::Substituted | OperandSource::Unknown => WordForm::Unproven,
    }
}

/// The selection the lowered case-list statement `stmt` makes when its
/// subject word holds `subject`: the words as the lowering recorded them
/// (`switch_arguments` over `raw_args` and the delimiter flags), through
/// [`literal_selection`] for the command the statement names. Only where the
/// plan reads the statement's own subject and clause count — an inliner
/// renames `subject` and leaves `raw_args` — and where the words'
/// delimiters were recorded; `None` otherwise.
pub(crate) fn statement_selection(
    registry: &CommandRegistry,
    stmt: &Statement,
    subject: &str,
) -> Option<SelectionFact> {
    let Statement::Switch {
        subject: written,
        arms,
        default_body,
        raw_args,
        raw_arg_braced,
        raw_arg_quoted,
        command,
        ..
    } = stmt
    else {
        return None;
    };
    if raw_arg_braced.len() != raw_args.len() || raw_arg_quoted.len() != raw_args.len() {
        return None;
    }
    let config = LexerConfig::for_profile(registry.profile());
    let cooked = switch_arguments(raw_args, (raw_arg_braced, raw_arg_quoted), &config);
    let words: Vec<(&str, WordForm)> = cooked
        .iter()
        .map(|word| (word.text.as_ref(), form_of(word.source)))
        .collect();
    let selection = literal_selection(registry, command, &words, Some(subject))?;
    let clauses = arms.len() + usize::from(default_body.is_some());
    (raw_args.get(selection.subject) == Some(written) && selection.clauses == clauses)
        .then_some(selection.fact)
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
            proven_pure_parameters: false,
        }),
        FoldPolicy::default(),
    );
    let node = crate::expr_parser::parse_expr_for_profile(text, None);
    let expression = ExpressionEvaluation {
        expression: Expression::Parsed(&node),
        policy: FoldPolicy::default(),
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
    driver.evaluate_expression_at(&expression, &uses, &values, &ssa)
}

#[cfg(test)]
mod tests {
    use super::*;

    thread_local! {
        /// The request size the next driver on this thread opens, when set.
        pub(super) static REQUEST_WORK: Cell<Option<u64>> = const { Cell::new(None) };
    }

    /// A recorded word has the value Tcl substitutes, or none: a bare or
    /// quoted word has its escapes decoded, and an escaped `$` or `[` is data;
    /// a braced word is its content with the line continuation collapsed and
    /// nothing else decoded; a word with a live substitution, which an escaped
    /// backslash before the `$` does not hide, has no value.
    #[test]
    fn a_recorded_word_has_the_value_tcl_substitutes() {
        let config = LexerConfig::default();
        let value = |text: &str, braced: bool| {
            recorded_word_value(text, braced, &config).map(Cow::into_owned)
        };
        for (text, braced, expected) in [
            (r"a\nb", false, Some("a\nb")),
            (r"a\tb", false, Some("a\tb")),
            (r"a\\b", false, Some(r"a\b")),
            (r"a\$b", false, Some("a$b")),
            (r"a\[b", false, Some("a[b")),
            (r"a\b", true, Some(r"a\b")),
            ("a\\\nb", true, Some("a b")),
            ("${x}", true, Some("${x}")),
            ("a${x}b", false, None),
            ("a[b]", false, None),
            (r"\\$x", false, None),
        ] {
            assert_eq!(
                value(text, braced).as_deref(),
                expected,
                "{text:?} braced: {braced}"
            );
        }
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

    /// A whole-array writer's evaluated element writes reach the lattice:
    /// `array set arr {k1 v1 k2 v2}` names `arr(k1)` and `arr(k2)`, which the
    /// SSA fans out as may-writes of the array's known elements, and each
    /// takes the written value rather than its join with the prior version.
    /// An element the pairs do not name stays a may-write and widens, and a
    /// dynamic-key write (`set arr($i) x`) still joins every element it may
    /// have hit with its prior.
    #[test]
    fn an_array_set_writes_the_elements_it_names() {
        let source = "proc p {i} {\n    set arr(k3) keep\n    array set arr {k1 v1 k2 v2}\n    \
                      puts \"$arr(k1) $arr(k2) $arr(k3)\"\n    set arr($i) x\n    puts $arr(k1)\n}\n";
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
            source, registry, false, "tcl8.6",
        );
        let function = unit.procedures.get("::p").expect("the procedure");
        let value = |name: &str, version: u32| {
            let symbol = function.ssa.var_symbol(name).expect("the variable");
            function.sccp.values.get(&(symbol, version)).cloned()
        };
        let text = |value: &str| Some(LatticeValue::Const(ConstValue::String(value.into())));
        assert_eq!(value("arr(k1)", 1), text("v1"));
        assert_eq!(value("arr(k2)", 1), text("v2"));
        assert_eq!(value("arr(k3)", 1), text("keep"));
        assert_eq!(
            value("arr(k3)", 2),
            Some(LatticeValue::Overdefined),
            "an element the pairs do not name is a may-write"
        );
        assert_ne!(
            value("arr(k1)", 2),
            text("v1"),
            "a dynamic key may have hit the element"
        );
    }

    /// A destructuring route resolves its targets' roles over the values the
    /// lattice proves: a substituted subject no
    /// longer hides them, so `regexp` writes its match variable, `lassign`
    /// both of its targets, and a no-match preserves its target — tclsh 8.5
    /// to 9.1 print `aa`, `1 2` and `before`.
    #[test]
    fn a_proven_subject_resolves_the_targets_roles() {
        let source = "proc p {} {\n    set s aab\n    regexp {(a+)b} $s -> g\n    \
                      set l {1 2}\n    lassign $l a b\n    set t zz\n    set m before\n    \
                      regexp {(x)} $t -> m\n    puts \"$g $a $b $m\"\n}\n";
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
            source, registry, false, "tcl8.6",
        );
        let function = unit.procedures.get("::p").expect("the procedure");
        let last = |name: &str| {
            let symbol = function.ssa.var_symbol(name).expect("the variable");
            function
                .sccp
                .values
                .iter()
                .filter(|((sym, _), _)| *sym == symbol)
                .max_by_key(|((_, version), _)| *version)
                .map(|(_, value)| value.clone())
        };
        let text = |value: &str| Some(LatticeValue::Const(ConstValue::String(value.into())));
        assert_eq!(last("g"), text("aa"));
        assert_eq!(last("a"), Some(LatticeValue::Const(ConstValue::Int(1))));
        assert_eq!(last("b"), Some(LatticeValue::Const(ConstValue::Int(2))));
        assert_eq!(last("m"), text("before"), "the no-match preserves `m`");
    }

    /// `proven_word_value` reads the lattice at the statement: a
    /// word reading a variable a `set` gave a literal is that literal, one
    /// reading a route's result carries the folded type the route states, a
    /// quoted word is its runs and reads concatenated and a literal word its
    /// text; the `$f` token is never the text `$f`. A word the lattice cannot
    /// pin is `None`: a command substitution, and the same `$f` after a
    /// branch redefines it.
    #[test]
    fn proven_word_value_reads_the_lattice_at_the_statement() {
        let source = "proc p {c} {\n    set f %d\n    set n [string length abc]\n    \
                      format $f $n \"x$f\" 7 [string length ab]\n    \
                      if {$c} { set f %s }\n    format $f 1\n}\n";
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let config = LexerConfig::for_profile(registry.profile());
        let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
            source, registry, false, "tcl8.6",
        );
        let function = unit.procedures.get("::p").expect("the procedure");
        // The span of `word` in the first place `call` is written.
        let word_at = |call: &str, word: &str| {
            let start = source.find(call).expect("the call") + call.find(word).expect("the word");
            function
                .word_at(Span::new(
                    u32::try_from(start).expect("offset"),
                    u32::try_from(start + word.len()).expect("offset"),
                ))
                .expect("a word of a reached call")
        };
        let (first, word) = word_at("format $f $n", "$f");
        assert_eq!(word, 1);
        let value = |statement: StatementId, word: usize| {
            proven_word_value(function, statement, word, config)
                .map(|(value, folded)| (String::from_utf8(value.bytes).expect("text"), folded))
        };
        assert_eq!(value(first, 1), Some(("%d".to_owned(), None)));
        let (length, folded) = value(first, 2).expect("the route's result");
        assert_eq!(length, "3");
        assert_eq!(
            folded.and_then(|folded| folded.intrep),
            Some(tcl_registry::TclType::Int)
        );
        assert_eq!(value(first, 3), Some(("x%d".to_owned(), None)));
        assert_eq!(value(first, 4), Some(("7".to_owned(), None)));
        assert_eq!(value(first, 5), None, "a command substitution");
        assert_eq!(value(first, 0), None, "the command word");
        let (second, word) = word_at("format $f 1", "$f");
        assert_eq!(word, 1);
        assert_ne!(second, first);
        assert_eq!(value(second, 1), None, "a branch redefines `f`");
    }

    /// A loop header binds each binder of its plan the elements it is
    /// assigned: `foreach {a b} {1 10 2 20 3} {…}` gives `a` the
    /// set `{1 2 3}` and `b` the set `{10 20 ""}`, the empty string where the
    /// last iteration runs past the list's end; a repeated binder holds its
    /// last position's element (`foreach {c c} {x y z w}` gives `c` `{y w}`).
    #[test]
    fn a_loop_header_binds_each_binder_its_elements() {
        let source = "proc p {} {\n    foreach {a b} {1 10 2 20 3} {\n        puts $a$b\n    }\n    \
                      foreach {c c} {x y z w} {\n        puts $c\n    }\n}\n";
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
            source, registry, false, "tcl8.6",
        );
        let function = unit.procedures.get("::p").expect("the procedure");
        let set_of = |texts: &[&str]| {
            LatticeValue::constset(
                texts
                    .iter()
                    .map(|text| exact_to_const(&ExactValue::from_literal(text)))
                    .collect(),
            )
        };
        let holds = |name: &str, expected: &LatticeValue| {
            let symbol = function.ssa.var_symbol(name).expect("the variable");
            function
                .sccp
                .values
                .iter()
                .any(|((sym, _), value)| *sym == symbol && value == expected)
        };
        assert!(
            holds("a", &set_of(&["1", "2", "3"])),
            "{:?}",
            function.sccp.values
        );
        assert!(
            holds("b", &set_of(&["10", "20", ""])),
            "{:?}",
            function.sccp.values
        );
        assert!(
            holds("c", &set_of(&["y", "w"])),
            "{:?}",
            function.sccp.values
        );
    }

    /// A dictionary body is found by its plan whatever its dictionary holds:
    /// the probe's dictionary holds the key path the plan reads,
    /// so `dict with d a b {…}` over a dictionary the analysis does not know
    /// is still one, under either spelling. Over a known dictionary the
    /// declared keys are read at the key path, which a dictionary lacking
    /// it fails as the command does, and a `dict update` variable is bound
    /// only when the dictionary holds its key.
    #[test]
    fn a_dictionary_body_is_found_by_its_plan() {
        let registry = CommandRegistry::build_default();
        let args =
            |words: &[&str]| -> Vec<String> { words.iter().map(|&word| word.to_owned()).collect() };
        let operand = |head: &str, words: &[&str]| dict_body_operand(&registry, head, &args(words));
        assert_eq!(operand("dict", &["with", "d", "{}"]), Some(1));
        assert_eq!(operand("dict", &["with", "d", "a", "b", "{}"]), Some(1));
        assert_eq!(operand("::tcl::dict::with", &["d", "a", "{}"]), Some(0));
        assert_eq!(operand("dict", &["update", "d", "k", "v", "{}"]), Some(1));
        assert_eq!(operand("dict", &["set", "d", "k", "v"]), None);
        assert_eq!(operand("foreach", &["x", "{1 2}", "{}"]), None);
        let key = |name: &str, value: &str| DictBinder::Key {
            name: name.to_owned(),
            value: value.to_owned(),
        };
        let body = |head: &str, words: &[&str], dictionary: &str| {
            dict_body(&registry, head, &args(words), dictionary)
        };
        assert_eq!(
            body("dict", &["with", "d", "{}"], "x 1 y 2 x 3"),
            Some(vec![key("x", "3"), key("y", "2")])
        );
        assert_eq!(
            body("dict", &["with", "d", "a", "{}"], "a {x 1} b {y 2}"),
            Some(vec![key("x", "1")])
        );
        assert_eq!(body("dict", &["with", "d", "z", "{}"], "a {x 1}"), None);
        assert_eq!(
            body(
                "dict",
                &["update", "d", "k", "v", "j", "w", "$m", "u", "{}"],
                "k 1 $m 2"
            ),
            Some(vec![
                DictBinder::Variable {
                    variable: 3,
                    bound: true
                },
                DictBinder::Variable {
                    variable: 5,
                    bound: false
                },
                DictBinder::Variable {
                    variable: 7,
                    bound: false
                },
            ])
        );
    }

    /// The driver records each executable `subst` call's template-word
    /// plan over the settled lattice: `set opt -novariables; subst
    /// $opt {hello $name}` reads the proven switch, so the template's
    /// `$name` is no read (tclsh 8.4 to 9.1 print `hello $name`); each record
    /// carries its template word's token span, the plan's own spans offsets
    /// into it; a call in a block the solver never reaches is not recorded.
    #[test]
    fn a_subst_call_records_its_template_plan() {
        use tcl_registry::substitution::SubstitutionKinds;
        let source = "proc p {} {\n    set opt -novariables\n    subst $opt {hello $name}\n    \
                      subst -nocommands {a$b}\n    if {0} { subst {dead $x} }\n}\n";
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
            source, registry, false, "tcl8.6",
        );
        let function = unit.procedures.get("::p").expect("the procedure");
        let records = &function.sccp.template_plans;
        let word = |text: &str| {
            let start = source.find(text).expect("the word");
            let at = |offset: usize| u32::try_from(offset).expect("a short source");
            Span::new(at(start), at(start + text.len()))
        };
        assert_eq!(
            records.iter().map(|record| record.span).collect::<Vec<_>>(),
            // A braced word's token runs from its `{` to its content's end.
            [word("{hello $name"), word("{a$b")],
            "{records:?}"
        );
        assert_eq!(
            records[0].plan.kinds,
            SubstitutionKinds {
                backslashes: true,
                commands: true,
                variables: false,
            }
        );
        assert!(records[0].plan.reads.is_empty(), "{records:?}");
        assert!(records[0].plan.braced && !records[0].plan.dynamic);
        let reads: Vec<(&str, Span)> = records[1]
            .plan
            .reads
            .iter()
            .map(|read| (read.name.as_str(), read.span))
            .collect();
        assert_eq!(reads, [("b", Span::new(2, 4))]);
    }

    /// The selection records of `::p` in `source` under `dialect`.
    fn selections_of(source: &str, dialect: &str) -> Vec<crate::sccp::SelectionRecord> {
        let registry = tcl_registry::model::ingress::static_context_for(dialect).commands();
        let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
            source, registry, false, dialect,
        );
        unit.procedures
            .get("::p")
            .expect("the procedure")
            .sccp
            .selections
            .clone()
    }

    /// The driver records each executable opaque case-list statement's
    /// selection over the settled lattice: the record's arm indices
    /// count the command's pattern and body pairs against the statement's
    /// arms, whose pattern spans it keeps in order, so the index one past
    /// them names the final `default` the statement keeps as its default
    /// body — `abc` selects `a*`, a literal `zzz` the default, and a
    /// finite subject one arm per member. A subject with no proven value,
    /// and a flattened exact switch, which is no statement at all, record
    /// nothing; `case`'s statement records through the command its binding
    /// site names. Each answer is tclsh 8.6.18's.
    #[test]
    fn an_opaque_switch_records_its_selection() {
        let source = "proc p {x} {\n\
                      set s abc\n\
                      switch -glob -- $s {a* {set r A} default {set r D}}\n\
                      switch -glob -- zzz {a* {set r A} default {set r D}}\n\
                      switch -glob -- $x {a* {set r A} default {set r D}}\n\
                      if {$x} {set t b} else {set t a}\n\
                      switch -glob -- $t {a {set r A} b {set r B} default {set r D}}\n\
                      switch -- $s {abc {set r A} default {set r D}}\n\
                      case abc in a* {set r A} default {set r D}\n\
                      }\n";
        let records = selections_of(source, "tcl8.6");
        let at = |nth: usize, text: &str| {
            let start = source.match_indices(text).nth(nth).expect("the word").0;
            let at = |offset: usize| u32::try_from(offset).expect("a short source");
            Span::new(at(start), at(start + text.len()))
        };
        assert_eq!(records.len(), 4, "{records:#?}");
        // `abc` selects the first arm, whose body runs.
        assert_eq!(records[0].fact.selected, [Some(0)]);
        assert_eq!(records[0].fact.bodies, [Some(0)]);
        assert_eq!(records[0].arm_pattern_spans, [at(0, "a*")]);
        assert!(!records[0].is_default(0));
        // A literal subject no pattern matches selects the default: the
        // index one past the kept arms.
        assert_eq!(records[1].fact.selected, [Some(1)]);
        assert_eq!(records[1].arm_pattern_spans, [at(1, "a*")]);
        assert!(records[1].is_default(1));
        // A finite subject records one arm per member.
        let mut members = records[2].fact.selected.clone();
        members.sort_unstable();
        assert_eq!(members, [Some(0), Some(1)]);
        assert_eq!(
            records[2].arm_pattern_spans,
            [at(0, "a {set r A} b"), at(0, "b {set r B}")]
                .map(|span| Span::new(span.start(), span.start() + 1))
        );
        // `case` records through its binding site; its statement spans the
        // whole command.
        assert_eq!(records[3].fact.selected, [Some(0)]);
        assert_eq!(records[3].span.start(), at(0, "case abc").start());
        assert!(
            records
                .iter()
                .all(|record| record.fact.writes.iter().all(Vec::is_empty))
        );
    }

    /// A delimited fall-through body reads two ways on 9.1b0, and
    /// the statement carries each word's delimiters to the transfer: `a`
    /// falls through a quoted or braced `-` into `b`'s body under 8.6 and
    /// 9.0, and no selection is recorded under a profile that may be 9.1;
    /// a bare `-` records under every one.
    #[test]
    fn a_delimited_fallthrough_body_records_no_selection_under_91() {
        for (body, under_91) in [("\"-\"", false), ("{-}", false), ("-", true)] {
            let source =
                format!("proc p {{}} {{\nset s a\nswitch -glob -- $s a {body} b {{set r B}}\n}}\n");
            for dialect in ["tcl8.6", "tcl9.0", "tcl9.1", "tcl"] {
                let records = selections_of(&source, dialect);
                let recorded = matches!(dialect, "tcl8.6" | "tcl9.0") || under_91;
                assert_eq!(records.len(), usize::from(recorded), "{dialect}: {body}");
                if recorded {
                    assert_eq!(records[0].fact.selected, [Some(0)], "{dialect}: {body}");
                    assert_eq!(records[0].fact.bodies, [Some(1)], "{dialect}: {body}");
                }
            }
        }
    }

    /// Each arm no member of the subject runs the body of is a `Selected`
    /// branch fact — no target, `false`, its pattern's text and span, and
    /// the block holding the statement — beside the record, and no
    /// reachability is applied for it: every block stays executable and no
    /// decided branch appears. The final `default` has no pattern, so it is
    /// never one; an arm a `-` body passes through to a running body is not.
    #[test]
    fn an_unreached_arm_is_a_selected_branch_fact() {
        use crate::sccp::BranchFactKind;
        let source = "proc p {} {\n\
                      set s abc\n\
                      switch -glob -- $s {a* {set r A} b* {set r B} default {set r D}}\n\
                      switch -glob -- $s {a* - z* {set r A} y* {set r B}}\n\
                      }\n";
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
            source, registry, false, "tcl8.6",
        );
        let function = unit.procedures.get("::p").expect("the procedure");
        let facts = &function.sccp.constant_branches;
        assert!(
            facts
                .iter()
                .all(|fact| fact.kind == BranchFactKind::Selected)
        );
        let patterns: Vec<&str> = facts.iter().map(|fact| fact.condition.as_str()).collect();
        // `abc` selects `a*` (and, through `-`, `z*`'s body) in the second
        // statement: only `y*` is never run there.
        assert_eq!(patterns, ["b*", "y*"], "{facts:#?}");
        let start = source.find("b* {").expect("the arm");
        let at = |offset: usize| u32::try_from(offset).expect("a short source");
        assert_eq!(facts[0].span, Some(Span::new(at(start), at(start + 2))));
        for fact in facts {
            assert!(
                !fact.value && fact.taken_target.is_empty() && fact.not_taken_target.is_empty()
            );
            assert!(
                function
                    .cfg
                    .block_by_name(&fact.block)
                    .is_some_and(|block| block
                        .statements
                        .iter()
                        .any(|statement| matches!(statement, Statement::Switch { .. }))),
                "the block holding the statement: {fact:?}"
            );
        }
        assert_eq!(
            function.sccp.executable_blocks.len(),
            function.cfg.blocks.len()
        );
    }

    /// A module that defines its own `switch` is not trusted to select as
    /// the builtin does: its opaque statement stays, and records nothing.
    #[test]
    fn a_redefined_switch_records_no_selection() {
        let source = "proc switch {args} {return x}\n\
                      proc p {} {\nswitch -glob -- abc {a* {set r A} default {set r D}}\n}\n";
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
            source, registry, false, "tcl8.6",
        );
        let function = unit.procedures.get("::p").expect("the procedure");
        assert!(
            function
                .ssa
                .blocks
                .values()
                .flat_map(|block| &block.statements)
                .any(|statement| matches!(statement.statement, Statement::Switch { .. })),
            "the opaque statement"
        );
        assert!(function.sccp.selections.is_empty());
    }

    /// The selection the call `head words…` makes under `dialect`.
    fn call_selection(
        dialect: &str,
        head: &str,
        words: &[(&str, WordForm)],
        subject: Option<&str>,
    ) -> Option<LiteralSelection> {
        let registry = tcl_registry::model::ingress::static_context_for(dialect).commands();
        literal_selection(registry, head, words, subject)
    }

    /// A consumer holding a call's words gets the command's own selection —
    /// mode, case folding, the final `default`, fall-through — from the
    /// registry, and the subject a caller proves for a word the call spells
    /// as a variable. Each answer is tclsh 8.6.18's.
    #[test]
    fn a_literal_call_selects_through_the_registry() {
        use WordForm::{Bare, Braced};
        let glob = [
            ("-glob", Bare),
            ("--", Bare),
            ("abc", Bare),
            ("a*", Bare),
            ("A", Braced),
            ("default", Bare),
            ("D", Braced),
        ];
        let chosen = call_selection("tcl8.6", "switch", &glob, None).expect("a selection");
        assert_eq!(chosen.fact.selected, [Some(0)]);
        assert_eq!(chosen.fact.bodies, [Some(0)]);
        assert_eq!((chosen.subject, chosen.clauses), (2, 2));
        // The subject the call spells as `$x`, given the value the caller
        // proves, selects the final default.
        let mut variable = glob;
        variable[2] = ("$x", WordForm::Unproven);
        assert!(call_selection("tcl8.6", "switch", &variable, None).is_none());
        let chosen = call_selection("tcl8.6", "switch", &variable, Some("zzz")).expect("selected");
        assert_eq!(chosen.fact.selected, [Some(1)]);
        assert_eq!(chosen.fact.bodies, [Some(1)]);
        // The regexp mode runs the engine; `-nocase` needs a release that has it.
        let regexp = [
            ("-regexp", Bare),
            ("--", Bare),
            ("abc", Bare),
            ("^a.c$", Bare),
            ("A", Braced),
        ];
        let chosen = call_selection("tcl8.6", "switch", &regexp, None).expect("a selection");
        assert_eq!(chosen.fact.selected, [Some(0)]);
        let nocase = [
            ("-nocase", Bare),
            ("--", Bare),
            ("ABC", Bare),
            ("abc", Bare),
            ("A", Braced),
        ];
        assert!(call_selection("tcl8.6", "switch", &nocase, None).is_some());
        assert!(call_selection("tcl", "switch", &nocase, None).is_none());
        // A pattern the caller does not prove declines the whole selection.
        let mut unproven = glob;
        unproven[3] = ("$p", WordForm::Unproven);
        assert!(call_selection("tcl8.6", "switch", &unproven, None).is_none());
    }

    /// A `-` body reads two ways on 9.1b0 when it is delimited: a bare one
    /// falls through everywhere, a quoted one only where the profile rules
    /// 9.1 out.
    #[test]
    fn a_delimited_fallthrough_word_declines_only_where_it_may_be_91() {
        use WordForm::{Bare, Braced, Quoted};
        for (form, under_91) in [(Bare, true), (Quoted, false), (Braced, false)] {
            let words = [
                ("--", Bare),
                ("a", Bare),
                ("a", Bare),
                ("-", form),
                ("b", Bare),
                ("B", Braced),
            ];
            for dialect in ["tcl8.6", "tcl9.0", "tcl9.1", "tcl"] {
                let chosen = call_selection(dialect, "switch", &words, None);
                let expected = matches!(dialect, "tcl8.6" | "tcl9.0") || under_91;
                assert_eq!(chosen.is_some(), expected, "{dialect}: {form:?}");
                if let Some(chosen) = chosen {
                    assert_eq!(chosen.fact.bodies, [Some(1)], "{dialect}: {form:?}");
                }
            }
        }
    }

    /// `case` reads its own contract through the same query: glob patterns
    /// and the `in` word skipped, from the releases that have it.
    #[test]
    fn a_literal_case_call_selects_its_glob_arm() {
        use WordForm::{Bare, Braced};
        let words = [
            ("abc", Bare),
            ("in", Bare),
            ("a*", Bare),
            ("A", Braced),
            ("default", Bare),
            ("D", Braced),
        ];
        for dialect in ["tcl8.4", "tcl8.6"] {
            let chosen = call_selection(dialect, "case", &words, None).expect("a selection");
            assert_eq!(chosen.fact.selected, [Some(0)], "{dialect}");
        }
        assert!(call_selection("tcl9.0", "case", &words, None).is_none());
    }

    /// The lowered statement's selection is asked over its recorded words,
    /// and only where they are the statement's own: a subject an inliner
    /// renamed, or words whose delimiters the lowering did not record, make
    /// none.
    #[test]
    fn a_lowered_statement_selects_over_its_recorded_words() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let module = crate::lowering::lower_to_ir(
            "switch -glob -- $mode {a* {set v 1} b {set v 2} default {set v 3}}\n",
            registry,
        );
        let statement = module.top_level.statements[0].clone();
        assert!(matches!(statement, Statement::Switch { .. }));
        let selected = |statement: &Statement, subject: &str| {
            statement_selection(registry, statement, subject).map(|fact| fact.bodies)
        };
        assert_eq!(selected(&statement, "abc"), Some(vec![Some(0)]));
        assert_eq!(selected(&statement, "b"), Some(vec![Some(1)]));
        assert_eq!(selected(&statement, "zzz"), Some(vec![Some(2)]));
        let mut renamed = statement.clone();
        if let Statement::Switch { subject, .. } = &mut renamed {
            *subject = "$other".to_owned();
        }
        assert_eq!(selected(&renamed, "abc"), None);
        let mut unrecorded = statement;
        if let Statement::Switch { raw_arg_quoted, .. } = &mut unrecorded {
            raw_arg_quoted.clear();
        }
        assert_eq!(selected(&unrecorded, "abc"), None);
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
                proven_pure_parameters: false,
            }),
            FoldPolicy::default(),
        );
        let node = crate::expr_parser::parse_expr_for_profile("abs([string length abc] - 5)", None);
        let expression = ExpressionEvaluation {
            expression: Expression::Parsed(&node),
            policy: FoldPolicy::default(),
            nested: NestedPolicy::EffectFreeOnly,
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

    /// Each nested write of an outcome as `(place, value)`, in order.
    type Written = Vec<(String, String)>;

    /// An evaluated answer's one outcome as its result and its nested
    /// writes.
    fn result_and_writes(answer: &LiftedAnswer) -> (String, Written) {
        let LiftedAnswer::Evaluated(outcomes) = answer else {
            panic!("the expression evaluates: {answer:?}");
        };
        let [outcome] = outcomes.as_slice() else {
            panic!("one outcome: {outcomes:?}");
        };
        let ExactValueOrUnavailable::Exact(result) = &outcome.result else {
            panic!("an exact result: {outcome:?}");
        };
        let writes = outcome
            .nested_writes
            .iter()
            .map(|(place, store)| match store {
                StoreOutcome::Write { value, .. } => (
                    place.name.clone(),
                    String::from_utf8_lossy(&value.bytes).into_owned(),
                ),
                other => panic!("a write: {other:?}"),
            })
            .collect();
        (String::from_utf8_lossy(&result.bytes).into_owned(), writes)
    }

    /// Under `LocalWrites` a nested write is applied to the evaluation's
    /// state in order, so the next read sees it: over `x` = 1,
    /// `$x + [incr x] + $x` reads 1, then 2 (the value is 5), and the state
    /// holds the one write. A branch the engine never reaches applies
    /// nothing, and the final value of a place written twice is the last.
    #[test]
    fn local_writes_apply_in_order() {
        let ordered =
            |text: &str| result_and_writes(&evaluate_over_x(text, NestedPolicy::LocalWrites));
        let write = |place: &str, value: &str| (place.to_owned(), value.to_owned());
        let cases: [(&str, &str, Written); 7] = [
            ("$x + [incr x] + $x", "5", vec![write("x", "2")]),
            ("0 && [incr x]", "0", vec![]),
            ("$x + [set x 10] + $x", "21", vec![write("x", "10")]),
            (
                "[incr x] + [incr x]",
                "5",
                vec![write("x", "2"), write("x", "3")],
            ),
            ("$x ? [incr x] : [incr x 10]", "2", vec![write("x", "2")]),
            ("!$x ? [incr x] : [incr x 10]", "11", vec![write("x", "11")]),
            // A quoted operand substitutes its variables and commands in
            // order under the same state.
            ("\"$x [incr x]\" eq {1 2}", "1", vec![write("x", "2")]),
        ];
        for (text, result, writes) in cases {
            assert_eq!(ordered(text), (result.to_owned(), writes), "{text}");
        }
    }

    /// The same programs under `EffectFreeOnly` decline as stateful where a
    /// nested command writes, and a write the state cannot own — a global —
    /// declines under `LocalWrites` too.
    #[test]
    fn a_nested_write_outside_the_state_declines() {
        for text in [
            "$x + [incr x] + $x",
            "$x + [set x 10] + $x",
            "[incr x] + [incr x]",
        ] {
            assert_eq!(
                evaluate_over_x(text, NestedPolicy::EffectFreeOnly),
                LiftedAnswer::Declined(DeclineReason::StatefulNested),
                "{text}"
            );
        }
        // Never reached: no write, so the effect-free policy answers.
        let (result, writes) = result_and_writes(&evaluate_over_x(
            "0 && [incr x]",
            NestedPolicy::EffectFreeOnly,
        ));
        assert_eq!((result.as_str(), writes.len()), ("0", 0));
        for policy in [NestedPolicy::EffectFreeOnly, NestedPolicy::LocalWrites] {
            assert_eq!(
                evaluate_over_x("$x + [incr ::g]", policy),
                LiftedAnswer::Declined(DeclineReason::StatefulNested),
                "{policy:?}"
            );
        }
    }

    /// `script` run as one command, under `policy`, by a detached driver over
    /// a frame where `x` holds 1, `s` holds `a` and `::g` is 5.
    fn run_over_frame(script: &str, policy: NestedPolicy) -> ScriptRun {
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
                proven_pure_parameters: false,
            }),
            FoldPolicy::default(),
        );
        let mut ssa = SsaFunction::trivial("::p", crate::cfg::BlockId(0), vec!["entry".into()]);
        let (x, s, g) = (
            ssa.intern_var("x"),
            ssa.intern_var("s"),
            ssa.intern_var("::g"),
        );
        let uses: HashMap<Symbol, Version> = HashMap::from([(x, 1), (s, 1), (g, 1)]);
        let values: HashMap<ValueKey, LatticeValue> = HashMap::from([
            ((x, 1), LatticeValue::Const(ConstValue::Int(1))),
            (
                (s, 1),
                LatticeValue::Const(ConstValue::String("a".to_owned())),
            ),
            ((g, 1), LatticeValue::Const(ConstValue::Int(5))),
        ]);
        driver
            .run_script(script, (&uses, &values, &ssa), (Vec::new(), policy))
            .expect("the script is one command the registry resolves")
    }

    /// A run's one outcome's result, and every write it places in order: the
    /// words' first, then the command's own.
    fn run_result_and_writes(run: &ScriptRun) -> (String, Written) {
        let (result, _) = result_and_writes(&run.answer);
        let [placed] = run.writes.as_slice() else {
            panic!("one outcome's writes: {:?}", run.writes);
        };
        let writes = placed
            .iter()
            .map(|(place, store)| match store {
                StoreOutcome::Write { value, .. } => (
                    place.name.clone(),
                    String::from_utf8_lossy(&value.bytes).into_owned(),
                ),
                other => panic!("a write: {other:?}"),
            })
            .collect();
        (result, writes)
    }

    /// A command's own substituting words run in order under one state before
    /// the command does, whatever route the command takes: what a word wrote
    /// is read by the command and is the first of the run's writes. `incr x
    /// [incr x]` over `x` = 1 runs the inner `incr` (2), then adds it (4) —
    /// reading `x` before the word ran adds it to 1 — and `string length
    /// [append s bc]` is the length of the new value, a route that keeps no
    /// ordered state of its own, with the append among its writes.
    #[test]
    fn a_commands_words_run_before_it_and_their_writes_are_the_first_of_its_own() {
        let write = |place: &str, value: &str| (place.to_owned(), value.to_owned());
        let cases: [(&str, &str, Written); 6] = [
            (
                "incr x [incr x]",
                "4",
                vec![write("x", "2"), write("x", "4")],
            ),
            ("string length [append s bc]", "3", vec![write("s", "abc")]),
            (
                "incr x \"[incr x]\"",
                "4",
                vec![write("x", "2"), write("x", "4")],
            ),
            (
                "append s [append s b]",
                "abab",
                vec![write("s", "ab"), write("s", "abab")],
            ),
            ("list $x [incr x] $x", "1 2 2", vec![write("x", "2")]),
            ("list [incr x] $x", "2 2", vec![write("x", "2")]),
        ];
        for (script, result, writes) in cases {
            let run = run_over_frame(script, NestedPolicy::LocalWrites);
            assert_eq!(
                run_result_and_writes(&run),
                (result.to_owned(), writes),
                "{script}"
            );
            assert_eq!(
                run_over_frame(script, NestedPolicy::EffectFreeOnly).answer,
                LiftedAnswer::Declined(DeclineReason::StatefulNested),
                "{script}"
            );
        }
    }

    /// The expression route's quoted operand is the word substituted before
    /// the engine parses it, once, under the same state as the commands in it:
    /// `expr "$x + [incr x]"` over `x` = 1 is `1 + 2`, however often the route
    /// reads the word, and leaves one write. An operand the engine substitutes
    /// itself is read after every word has run: `expr {$x} + [incr x]` joins
    /// its words into `$x + 2` and reads `x` then, so it is `2 + 2`.
    #[test]
    fn a_quoted_expr_operand_is_substituted_once_under_the_ordered_state() {
        let write = |place: &str, value: &str| (place.to_owned(), value.to_owned());
        for (script, result) in [
            ("expr \"$x + [incr x]\"", "3"),
            ("expr \"[incr x] + 0\"", "2"),
            ("expr \"[incr x]\"", "2"),
            ("expr {$x} + [incr x]", "4"),
        ] {
            let run = run_over_frame(script, NestedPolicy::LocalWrites);
            assert_eq!(
                run_result_and_writes(&run),
                (result.to_owned(), vec![write("x", "2")]),
                "{script}"
            );
        }
        let run = run_over_frame("expr \"$x + [incr x]\"", NestedPolicy::EffectFreeOnly);
        assert_eq!(
            run.answer,
            LiftedAnswer::Declined(DeclineReason::StatefulNested)
        );
    }

    /// The definitions an embedded call takes from its host's writes: a place
    /// no definition belongs to, and a cell written beside the array that
    /// holds it, are writes no definition carries, and a place no write names
    /// keeps what it held.
    #[test]
    fn the_definitions_decline_a_write_none_of_them_can_carry() {
        let driver = LatticeDriver::detached(None, FoldPolicy::default());
        let mut ssa = SsaFunction::trivial("::p", crate::cfg::BlockId(0), vec!["entry".into()]);
        let (x, cell, array) = (
            ssa.intern_var("x"),
            ssa.intern_var("a(1)"),
            ssa.intern_var("a"),
        );
        let uses: HashMap<Symbol, Version> = HashMap::from([(x, 1)]);
        let values: HashMap<ValueKey, LatticeValue> =
            HashMap::from([((x, 1), LatticeValue::Const(ConstValue::Int(1)))]);
        let inputs = driver.expression_inputs((&uses, &values, &ssa));
        let defs = vec![
            ("x".to_owned(), (x, 2)),
            ("a(1)".to_owned(), (cell, 2)),
            ("a".to_owned(), (array, 2)),
        ];
        let write = |name: &str, text: &str| {
            (
                place_named(name),
                StoreOutcome::Write {
                    target: TargetId(OperandId(0)),
                    value: ExactValue::from_literal(text),
                },
            )
        };
        assert!(
            driver
                .embedded_defs(&[write("y", "1")], &defs, &inputs)
                .is_none(),
            "a place no definition belongs to"
        );
        assert!(
            driver
                .embedded_defs(&[write("a(1)", "1"), write("a", "2")], &defs, &inputs)
                .is_none(),
            "a cell and its array"
        );
        let answers = driver
            .embedded_defs(&[write("a(1)", "9")], &defs, &inputs)
            .expect("a write a definition carries");
        assert_eq!(answers[0].value, LatticeValue::Const(ConstValue::Int(1)));
        assert!(answers[0].preserved && answers[0].stated);
        assert_eq!(
            answers[1].value,
            exact_to_lattice(&ExactValue::from_literal("9"))
        );
        assert!(!answers[1].preserved && answers[1].stated);
    }

    /// Inputs whose nested script `boom` raises an error after the writes
    /// made so far, and which answer every other request through `inner`.
    struct RaisesOnBoom<'a>(&'a dyn AnalysisInputs);

    impl AnalysisInputs for RaisesOnBoom<'_> {
        fn invocation(&self) -> &ResolvedInvocationView<'_> {
            self.0.invocation()
        }

        fn operand(&self, id: OperandId, domain: FactDomain) -> FactView {
            self.0.operand(id, domain)
        }

        fn place(&self, id: OperandId) -> Result<PlaceRef, DeclineReason> {
            self.0.place(id)
        }

        fn variable(&self, name: &str, domain: FactDomain) -> FactView {
            self.0.variable(name, domain)
        }

        fn prior_store(&self, place: &PlaceRef, domain: FactDomain) -> FactView {
            self.0.prior_store(place, domain)
        }

        fn word_structure(&self, id: OperandId) -> Result<WordStructure, DeclineReason> {
            self.0.word_structure(id)
        }

        fn body(&self, id: OperandId) -> Result<BodyRegion, DeclineReason> {
            self.0.body(id)
        }

        fn nested(&self, script: &str, state: &mut EvaluationState) -> EvalAnswer {
            if script != "boom" {
                return self.0.nested(script, state);
            }
            EvalAnswer::Evaluated(Box::new(InvocationOutcome {
                completion: CompletionOutcome::Error {
                    written: 0,
                    message: ExactValueOrUnavailable::Exact(ExactValue::text("mid")),
                    error_code: ExactValueOrUnavailable::Exact(ExactValue::text("NONE")),
                },
                nested_writes: Vec::new(),
                result: ExactValueOrUnavailable::Exact(ExactValue::text("")),
                ordered_stores: Vec::new(),
                types: TypeFacts::default(),
                evidence: DependencyEvidence::default(),
            }))
        }

        fn math_function(&self, name: &str) -> Result<BindingIdentity, DeclineReason> {
            self.0.math_function(name)
        }

        fn context(&self) -> &AnalysisContext {
            self.0.context()
        }
    }

    /// An error completion ends the evaluation with that completion and the
    /// writes so far: `[incr x] + [boom]` over `x` = 1 leaves the increment
    /// in the state and no value, and the error names the writes that ran
    /// before it. The lattice publishes nothing for it: the outcome has no
    /// exact result.
    #[test]
    fn an_error_completion_ends_the_evaluation_with_the_writes_so_far() {
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
                proven_pure_parameters: false,
            }),
            FoldPolicy::default(),
        );
        let node = crate::expr_parser::parse_expr_for_profile("[incr x] + [boom] + [incr x]", None);
        let expression = ExpressionEvaluation {
            expression: Expression::Parsed(&node),
            policy: FoldPolicy::default(),
            nested: NestedPolicy::LocalWrites,
            head: None,
        };
        let mut ssa = SsaFunction::trivial("::p", crate::cfg::BlockId(0), vec!["entry".into()]);
        let x = ssa.intern_var("x");
        let uses: HashMap<Symbol, Version> = HashMap::from([(x, 1)]);
        let values: HashMap<ValueKey, LatticeValue> =
            HashMap::from([((x, 1), LatticeValue::Const(ConstValue::Int(1)))]);
        let inputs = LatticeInputs {
            driver: &driver,
            prior_writes: Vec::new(),
            words: Words::Independent,
            view: ResolvedInvocationView {
                canonical_command: "expr",
                subcommand: None,
                form: None,
                layout: InvocationLayout::Source,
                operands: Vec::new(),
                argument_offset: 0,
                arity: None,
            },
            uses: &uses,
            values: &values,
            ssa: &ssa,
            sources: Vec::new(),
        };
        let EvalAnswer::Evaluated(outcome) =
            expression.evaluate(&RaisesOnBoom(&inputs), &mut driver.budget())
        else {
            panic!("the evaluation ends with an outcome");
        };
        assert!(
            matches!(
                &outcome.completion,
                CompletionOutcome::Error { written: 1, message: ExactValueOrUnavailable::Exact(m), .. }
                    if m.bytes == b"mid"
            ),
            "{:?}",
            outcome.completion
        );
        assert!(matches!(
            outcome.result,
            ExactValueOrUnavailable::Unavailable(_)
        ));
        let writes: Vec<(&str, &StoreOutcome)> = outcome
            .nested_writes
            .iter()
            .map(|(place, store)| (place.name.as_str(), store))
            .collect();
        assert!(
            matches!(
                writes.as_slice(),
                [("x", StoreOutcome::Write { value, .. })] if value.as_int() == Some(2)
            ),
            "{writes:?}"
        );
    }

    /// An integer division or remainder by zero is an error completion with
    /// the wording every release shares — `divide by zero`, `-errorcode`
    /// `ARITH DIVZERO {divide by zero}` — and the writes the expression's own
    /// commands made before it are counted. A float operand makes the
    /// operation floating point, which does not raise, and an operand that is
    /// no integer raises another error the engine does not word.
    #[test]
    fn an_integer_division_by_zero_is_an_error_completion() {
        let completion_of = |text: &str| {
            let LiftedAnswer::Evaluated(outcomes) =
                evaluate_over_x(text, NestedPolicy::LocalWrites)
            else {
                panic!("{text} evaluates");
            };
            let [outcome] = outcomes.as_slice() else {
                panic!("one outcome for {text}: {outcomes:?}");
            };
            (outcome.completion.clone(), outcome.nested_writes.len())
        };
        let divide_by_zero = |written: usize| CompletionOutcome::Error {
            written,
            message: ExactValueOrUnavailable::exact_text("divide by zero"),
            error_code: ExactValueOrUnavailable::exact_text("ARITH DIVZERO {divide by zero}"),
        };
        for (text, written) in [
            ("1 / 0", 0),
            ("$x % 0", 0),
            ("0 / 0", 0),
            ("[incr x] / 0", 1),
            ("[incr x] + [incr x] % 0", 2),
        ] {
            let (completion, writes) = completion_of(text);
            assert_eq!(completion, divide_by_zero(written), "{text}");
            assert_eq!(writes, written, "{text}");
        }
        for text in ["1 / 0.0", "1.0 / 0", "$x / 2", "1 % 3"] {
            let (completion, _) = completion_of(text);
            assert_eq!(completion, CompletionOutcome::Normal, "{text}");
        }
        assert!(
            matches!(
                evaluate_over_x("{a} / 0", NestedPolicy::LocalWrites),
                LiftedAnswer::Declined(_)
            ),
            "no integer is divided"
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

    /// `simple_var_ref_name` reads exactly one reference. A lowered
    /// quoted `expr` word spells `${a} + ${b}`, which starts and ends like one
    /// braced reference and is not the variable `a} + ${b`. The `${…}`
    /// closer is the release rule's: `${a{b}c}` is one reference
    /// under 9.x and `${a{b}` followed by `c}` under 8.x.
    #[test]
    fn a_simple_reference_is_one_reference_only() {
        use tcl_dialect::BracedVarStyle::{FirstClose, Tcl9Nesting};
        for style in [Tcl9Nesting, FirstClose] {
            assert_eq!(simple_var_ref_name("$a", style), Some("a"));
            assert_eq!(simple_var_ref_name("${a b}", style), Some("a b"));
            assert_eq!(simple_var_ref_name("$::ns::v", style), Some("::ns::v"));
            assert_eq!(simple_var_ref_name("${a} + ${b}", style), None);
            assert_eq!(simple_var_ref_name("$a + $b", style), None);
            assert_eq!(simple_var_ref_name("a", style), None);
        }
        assert_eq!(simple_var_ref_name("${a{b}c}", Tcl9Nesting), Some("a{b}c"));
        assert_eq!(simple_var_ref_name("${a{b}c}", FirstClose), None);
        assert_eq!(simple_var_ref_name("${a\\}", FirstClose), Some("a\\"));
        assert_eq!(simple_var_ref_name("${a\\}", Tcl9Nesting), None);
    }

    /// A `Raw` operand reads a variable only when it is one reference whose
    /// name the release rules read alike: `${acc}` and `$acc` do, a
    /// backslash or brace in the name never does, and neither does any
    /// other `Raw` text.
    #[test]
    fn a_raw_operand_is_a_variable_only_when_every_rule_agrees() {
        use tcl_dialect::BracedVarStyle::{FirstClose, Tcl9Nesting};
        for style in [Tcl9Nesting, FirstClose] {
            assert_eq!(whole_variable_operand("${acc}", style), Some("acc"));
            assert_eq!(whole_variable_operand("$acc", style), Some("acc"));
            assert_eq!(whole_variable_operand("${a\\b}", style), None);
            assert_eq!(whole_variable_operand("<switch_jump>", style), None);
            assert_eq!(whole_variable_operand("$a + $b", style), None);
        }
        assert_eq!(whole_variable_operand("${a{b}c}", Tcl9Nesting), None);
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
