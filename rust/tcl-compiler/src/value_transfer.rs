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
//! Some declared direct routes are still implemented here rather than in
//! the registry ([`NativeEvalId::owner`] says which): the compiler's
//! transitional handlers, each carried verbatim from the arm it replaces so
//! the lattice stays byte-identical, and each listed with its expiry in the
//! migration plan's ledger. The expression route is run here by
//! construction — the shared engine is fed by this module's lattice
//! services — and its registry-owned argument assembly lands with the
//! expression slice.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use rustc_hash::FxHashSet;
use tcl_dialect::StringCharacterModel;
use tcl_lexer::{LexerConfig, Span, TokenType};
use tcl_registry::value_transfer::{
    AnalysisContext, AnalysisInputs, AnalysisTier, BindingIdentity, BodyRegion, Budget,
    DeclineReason, EvalAnswer, EvalRoute, EvaluationState, EvaluatorOwner, ExactValue,
    ExactValueOrUnavailable, ExistenceOutcome, FactDomain, FactView, InvocationLayout,
    InvocationOutcome, IterableKind, LiftedAnswer, NativeEvalId, NumericValue, OperandId,
    OperandView, PlaceKind, PlaceRef, PlanAnswer, ResolvedInvocationView, StoreOutcome, TargetId,
    TransferAnswer, ValueIdentity, WordStructure, evaluate_lifted,
};
use tcl_registry::{
    ArgRole, CommandRegistry, InvocationWord, InvocationWordKind, InvocationWords,
    ResolvedInvocation,
};

use crate::analyses::{ConstValue, LatticeValue, MAX_CONSTSET_SIZE};
use crate::cfg::Function as CfgFunction;
use crate::codegen::helpers::split_list_values;
use crate::command_binding::CommandTrustSnapshot;
use crate::ir::Statement;
use crate::sccp::{
    BuiltinFoldInputs, TraceInputs, extract_foreach_elements, resolve_foreach_list_via_lattice,
};
use crate::ssa::{SsaFunction, SsaStatement, Symbol, ValueKey, Version};
use crate::tcl_expr_eval::{FoldPolicy, eval_tcl_expr_with_policy};

/// The analysis context's hashable identity, as a per-function memo key
/// carries it: the facts that can change an answer and that the key's
/// other fields (body, dialect, grammar, traces, seeds) do not already
/// cover. Built once per module and shared by every function in it, so a
/// `rename` anywhere in the module re-keys every function's lattice — the
/// sensitivity the design asks for.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AnalysisContextKey {
    /// The effective registry generation. The un-overlaid registry a
    /// dialect resolves to is built once per process, so this is fixed
    /// until the overlay generation reaches the per-function key.
    pub registry_generation: u64,
    /// The workspace pack overlay generation, once it reaches the key.
    pub overlay_generation: Option<u64>,
    /// The module's command-binding evidence.
    pub bindings: CommandTrustSnapshot,
    /// The precision tier the request runs at.
    pub tier: AnalysisTier,
    /// Evaluator and implementation revisions the registry does not fix.
    /// No evaluator host reaches the lattice yet, so nothing bumps it.
    pub evaluator_revision: u64,
}

impl AnalysisContextKey {
    /// The key for a module whose command mutations were scanned.
    #[must_use]
    pub fn for_module(mutations: &crate::command_binding::ModuleCommandMutations) -> Self {
        Self {
            registry_generation: 0,
            overlay_generation: None,
            bindings: mutations.snapshot(),
            tier: AnalysisTier::Deep,
            evaluator_revision: 0,
        }
    }

    /// The key for a consumer with no module view: no mutations observed.
    #[must_use]
    pub fn detached() -> Self {
        Self::for_module(&crate::command_binding::ModuleCommandMutations::default())
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
    /// The statement being evaluated, when the solver said which.
    explaining: Cell<Option<Span>>,
    /// The last explanation recorded per statement.
    explanations: RefCell<BTreeMap<(u32, u32), RouteExplanation>>,
}

/// The lattice value of one member-wise evaluation: the values `pick`
/// selects from each outcome, one constant or a finite set of them, and
/// `Overdefined` when any outcome has none.
fn lattice_of_outcomes(
    outcomes: &[Box<InvocationOutcome>],
    pick: impl Fn(&InvocationOutcome) -> Option<&ExactValue>,
) -> LatticeValue {
    let mut consts: Vec<ConstValue> = Vec::with_capacity(outcomes.len());
    for outcome in outcomes {
        let Some(value) = pick(outcome) else {
            return LatticeValue::Overdefined;
        };
        let LatticeValue::Const(c) = exact_to_lattice(value) else {
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

/// The value an outcome writes to `target`, when it writes one.
fn written_value(outcome: &InvocationOutcome, target: TargetId) -> Option<&ExactValue> {
    outcome.ordered_stores.iter().find_map(|store| match store {
        StoreOutcome::Write { target: t, value } if *t == target => Some(value),
        _ => None,
    })
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
            format!("implementation {}", capability.identity)
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
        Self {
            registry,
            folds,
            policy,
            context,
            lexer_config: LexerConfig::for_profile(profile),
            explaining: Cell::new(None),
            explanations: RefCell::new(BTreeMap::new()),
        }
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

    /// Record whether a route the driver runs itself folded the statement.
    fn explain_fold(&self, command: &str, route: EvalRoute, folded: Option<&LatticeValue>) {
        let answer = if folded.is_some() {
            "evaluated"
        } else {
            "declined: unsupported"
        };
        self.explain(command, Some(route), answer.to_owned());
    }

    /// Every explanation the run recorded, in statement order.
    pub(crate) fn take_explanations(&self) -> Vec<RouteExplanation> {
        std::mem::take(&mut *self.explanations.borrow_mut())
            .into_values()
            .collect()
    }

    /// The per-evaluation budget a route runs under.
    fn budget() -> Budget {
        Budget::evaluation()
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

    /// The fold policy this run evaluates under.
    pub(crate) const fn policy(&self) -> FoldPolicy {
        self.policy
    }

    /// Binding validity for `head`: with the whole-module trust fact, the
    /// name must still denote its registry command everywhere in the
    /// module; without it — the mutation-fact-free shared lattice, which
    /// no rewrite lands from — every binding is trusted, as it always was.
    fn trusted(&self, head: &str) -> bool {
        self.folds.is_none_or(|f| f.mutations.trusts(head))
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

    /// The `incr name ?amount?` statement: its typed node projects to the
    /// invocation view `incr name ?amount?`, and the registry's derived
    /// cell update evaluates it.
    pub(crate) fn evaluate_incr<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &self,
        name: &str,
        amount: Option<&str>,
        uses: &HashMap<Symbol, Version, S1>,
        values: &HashMap<ValueKey, LatticeValue, S2>,
        ssa: &SsaFunction,
    ) -> LatticeValue {
        const HEAD: &str = "incr";
        if !self.trusted(HEAD) {
            return LatticeValue::Overdefined;
        }
        let texts: Vec<&str> = std::iter::once(name).chain(amount).collect();
        let words: Vec<InvocationWord<'_>> = std::iter::once(InvocationWord::Literal(name))
            .chain(amount.map(word_of))
            .collect();
        let Some(resolved) = self.resolve(HEAD, &words) else {
            return LatticeValue::Overdefined;
        };
        let Some(semantics) = resolved.semantics.value.semantics() else {
            return LatticeValue::Overdefined;
        };
        let view = view_of(&resolved, &texts, &words, InvocationLayout::Source);
        let inputs = LatticeInputs {
            driver: self,
            view,
            uses,
            values,
            ssa,
        };
        self.cell_update_def(HEAD, semantics, &inputs)
    }

    /// The lattice value a resolved cell update writes to its target:
    /// the registry's evaluator over the lattice inputs, lifted over one
    /// finite input, on the route the registry owns.
    fn cell_update_def(
        &self,
        head: &str,
        semantics: &dyn tcl_registry::value_transfer::CommandSemantics,
        inputs: &dyn AnalysisInputs,
    ) -> LatticeValue {
        let PlanAnswer::CellReadModifyWrite { target, .. } = semantics.structure(inputs) else {
            self.explain(
                head,
                Some(semantics.route()),
                "no cell update for this shape".to_owned(),
            );
            return LatticeValue::Overdefined;
        };
        let route = semantics.route();
        if !matches!(route, EvalRoute::Direct { id } if id.owner() == EvaluatorOwner::Registry) {
            self.explain(
                head,
                Some(route),
                "not evaluated: the route is not registry-owned".to_owned(),
            );
            return LatticeValue::Overdefined;
        }
        let answer = evaluate_lifted(semantics, inputs, &mut Self::budget(), MAX_CONSTSET_SIZE);
        self.explain(head, Some(route), answer_label(&answer));
        match answer {
            LiftedAnswer::Pending => LatticeValue::Unknown,
            LiftedAnswer::Declined(_) => LatticeValue::Overdefined,
            LiftedAnswer::Evaluated(outcomes) => {
                lattice_of_outcomes(&outcomes, |outcome| written_value(outcome, target))
            }
        }
    }

    /// A `Call` statement's defs. Two call shapes have a declared structure
    /// the solver applies: the synthetic loop header the CFG builder emits
    /// — the one `Call` carrying `foreach_groups` — whose iteration plan's
    /// single list binder takes the set of the list's elements, and a call
    /// whose resolved plan is a cell read-modify-write (`append`,
    /// `lappend`), whose target takes the registry's evaluated store. Every
    /// other call keeps the conservative answer.
    pub(crate) fn evaluate_call<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &self,
        stmt_ssa: &SsaStatement,
        values: &HashMap<ValueKey, LatticeValue, S2>,
        ssa: &SsaFunction,
        uses: &HashMap<Symbol, Version, S1>,
    ) -> LatticeValue {
        let Statement::Call {
            args,
            defs,
            foreach_groups,
            ..
        } = &stmt_ssa.statement
        else {
            return LatticeValue::Overdefined;
        };
        let head = stmt_ssa.statement.canonical_command_or_source();
        if !self.trusted(head) {
            self.explain(head, None, "declined: rebinding-suspected".to_owned());
            return LatticeValue::Overdefined;
        }
        let texts: Vec<&str> = args.iter().map(String::as_str).collect();
        let words: Vec<InvocationWord<'_>> = texts.iter().copied().map(word_of).collect();
        let Some(resolved) = self.resolve(head, &words) else {
            return LatticeValue::Overdefined;
        };
        let Some(semantics) = resolved.semantics.value.semantics() else {
            self.explain(head, None, "declined: no-semantics".to_owned());
            return LatticeValue::Overdefined;
        };
        if foreach_groups.is_none() {
            let view = view_of(&resolved, &texts, &words, InvocationLayout::Source);
            let inputs = LatticeInputs {
                driver: self,
                view,
                uses,
                values,
                ssa,
            };
            return self.cell_update_def(head, semantics, &inputs);
        }
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
                    && let Some(LatticeValue::Const(ConstValue::String(s))) =
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

    /// Fold a `[cmd args…]` command substitution through the resolved
    /// command's declared route, then — when the caller holds the trust
    /// fact — through the registry's constant-fold engine.
    pub(crate) fn fold_cmd_subst<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &self,
        value: &str,
        uses: &HashMap<Symbol, Version, S1>,
        values: &HashMap<ValueKey, LatticeValue, S2>,
        ssa: &SsaFunction,
    ) -> Option<LatticeValue> {
        if let Some(folded) = self.fold_cmd_subst_routes(value, uses, values, ssa) {
            return Some(folded);
        }
        // Registry const-fold fallback: the fold's `$var` words resolve at
        // this statement's use versions, so a folded value re-enters the
        // lattice and downstream statements see it — the multi-hop chain
        // the declared routes cannot close yet. Checked after them so
        // single-hop results stay byte-identical.
        let f = self.folds?;
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
        Some(exact_to_lattice(&ExactValue::from_literal(&folded)))
    }

    /// The declared-route half of [`Self::fold_cmd_subst`]: resolve the
    /// head, ask the registry which route the invocation declares, and run
    /// it — a transitional handler, the expression engine, or a
    /// registry-owned evaluator under the effect-free nested policy.
    fn fold_cmd_subst_routes<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &self,
        value: &str,
        uses: &HashMap<Symbol, Version, S1>,
        values: &HashMap<ValueKey, LatticeValue, S2>,
        ssa: &SsaFunction,
    ) -> Option<LatticeValue> {
        let inner = value.strip_prefix('[')?.strip_suffix(']')?;
        let (head, rest) = split_head(inner);
        // Binding validity comes first: after `rename list mylist` or a
        // shadowing `proc format …` anywhere in the unit, `[list a 1]` is a
        // call to something else entirely.
        if !self.trusted(head) {
            return None;
        }
        let commands =
            crate::segmenter::segment_commands_with_offset_and_config(inner, 0, self.lexer_config);
        let [seg] = commands.as_slice() else {
            return None;
        };
        if seg.name() != head {
            return None;
        }
        let texts: Vec<&str> = seg.args().iter().map(String::as_str).collect();
        let words: Vec<InvocationWord<'_>> = seg
            .arg_tokens()
            .iter()
            .zip(seg.arg_single_token())
            .zip(&texts)
            .map(|((token, single), text)| match (single, token.kind) {
                (true, TokenType::Str | TokenType::Esc) => InvocationWord::Literal(text),
                (_, TokenType::Expand) => InvocationWord::Expanded,
                _ => InvocationWord::Dynamic,
            })
            .collect();
        let resolved = self.resolve(head, &words)?;
        let semantics = resolved.semantics.value.semantics()?;
        let view = view_of(&resolved, &texts, &words, InvocationLayout::Source);
        let inputs = LatticeInputs {
            driver: self,
            view,
            uses,
            values,
            ssa,
        };
        let route = semantics.route();
        match route {
            EvalRoute::Direct { id } => match id.owner() {
                EvaluatorOwner::Transitional { .. } => {
                    let folded = self.transitional_direct(id, value, rest, uses, values, ssa);
                    self.explain_fold(head, route, folded.as_ref());
                    folded
                }
                EvaluatorOwner::Registry => {
                    let answer =
                        evaluate_lifted(semantics, &inputs, &mut Self::budget(), MAX_CONSTSET_SIZE);
                    // In value position a nested invocation runs under the
                    // effect-free policy: an outcome with stores has no
                    // definition to land on here.
                    let folded = match &answer {
                        LiftedAnswer::Evaluated(outcomes)
                            if outcomes.iter().all(|outcome| !outcome.has_stores()) =>
                        {
                            Some(lattice_of_outcomes(outcomes, |outcome| {
                                match &outcome.result {
                                    ExactValueOrUnavailable::Exact(value) => Some(value),
                                    ExactValueOrUnavailable::Unavailable(_) => None,
                                }
                            }))
                            .filter(|folded| *folded != LatticeValue::Overdefined)
                        }
                        LiftedAnswer::Pending => Some(LatticeValue::Unknown),
                        LiftedAnswer::Evaluated(_) | LiftedAnswer::Declined(_) => None,
                    };
                    self.explain(
                        head,
                        Some(route),
                        match (&answer, &folded) {
                            (LiftedAnswer::Evaluated(_), None) => {
                                "not substituted: the outcome writes storage".to_owned()
                            }
                            _ => answer_label(&answer),
                        },
                    );
                    folded
                }
            },
            EvalRoute::Expression { .. } => {
                let folded = self.expression_route(rest, uses, values, ssa);
                self.explain_fold(head, route, folded.as_ref());
                folded
            }
            EvalRoute::Implementation(_) | EvalRoute::None { .. } => {
                self.explain(head, Some(route), "declined: no-route".to_owned());
                None
            }
        }
    }

    /// The transitional compiler-owned direct evaluators
    /// (`docs/design/compiler/value-transfers-migration.md`, the ledger).
    /// Each arm is one command's fold carried verbatim from the arm it
    /// replaces, keyed by the registry's evaluator id rather than by the
    /// command's name, until the shared cores retire it.
    fn transitional_direct<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &self,
        id: NativeEvalId,
        value: &str,
        rest: Option<&str>,
        uses: &HashMap<Symbol, Version, S1>,
        values: &HashMap<ValueKey, LatticeValue, S2>,
        ssa: &SsaFunction,
    ) -> Option<LatticeValue> {
        let policy = self.policy;
        // value-transfer-ok: dataflow — the transitional direct evaluators
        // of the migration ledger, keyed by `NativeEvalId`; slice 2 moves
        // each onto the shared cores and deletes this table.
        match id {
            // `[list ...]` — reuse the codegen fold.
            NativeEvalId::ListOfArgs => {
                crate::codegen::helpers::fold_list_cmd(value, policy.word_rules)
                    .map(|folded| LatticeValue::Const(ConstValue::String(folded)))
            }
            // `[format "..." args…]` with literal args. The document's
            // escape grammar comes from the same resolved profile the rest
            // of the policy's axes come from; a caller with no dialect keeps
            // the 9.0 default.
            NativeEvalId::FormatTemplate => {
                let escapes = policy
                    .dialect
                    .map_or_else(tcl_dialect::EscapeSyntax::default, |profile| {
                        profile.grammar.escapes
                    });
                crate::codegen::helpers::try_format_fold(value, escapes)
                    .map(|folded| LatticeValue::Const(ConstValue::String(folded)))
            }
            // `[llength LIST]` with a literal or lattice-resolvable list.
            NativeEvalId::ListLength => {
                let arg = rest?.trim();
                // Unlike a loop header's list (already delimiter-stripped
                // by the segmenter), `arg` is raw source text straight out
                // of the `[...]` substitution, so it still carries its own
                // `{…}` / `"…"` wrapping — peel exactly one level before
                // splitting.
                if let Some(elements) =
                    extract_foreach_elements(strip_one_level(arg), policy.word_rules)
                {
                    let n = i64::try_from(elements.len()).unwrap_or(i64::MAX);
                    return Some(LatticeValue::Const(ConstValue::Int(n)));
                }
                let items =
                    resolve_foreach_list_via_lattice(arg, uses, values, ssa, policy.word_rules)?;
                let n = i64::try_from(items.len()).unwrap_or(i64::MAX);
                Some(LatticeValue::Const(ConstValue::Int(n)))
            }
            // `[string length OPERAND]` where OPERAND resolves to a constant
            // string — a literal word, or a `$var` whose lattice value is
            // known. `string length` counts UTF-16 code units on Tcl 8 and
            // Unicode scalars on Tcl 9, so the fold uses the selected
            // dialect's model; with no selected release the count survives
            // only where both models agree.
            NativeEvalId::StringLength => {
                let (_, sub_rest) = split_head(rest?.trim());
                let s = resolve_const_string(sub_rest?.trim(), uses, values, ssa)?;
                let count = StringCharacterModel::count_for(policy.characters, &s)?;
                let len = i64::try_from(count).unwrap_or(i64::MAX);
                Some(LatticeValue::Const(ConstValue::Int(len)))
            }
            // Registry-owned: never a transitional handler.
            NativeEvalId::CellIncrement
            | NativeEvalId::CellAppend
            | NativeEvalId::CellListAppend
            | NativeEvalId::StringRange => None,
        }
    }

    /// The expression route as the driver runs it: parse the argument and
    /// fold it under the current lattice. Braced (`expr {…}`) versus quoted
    /// or bare (`expr "…"`, `expr …`) changes the substitution model: in a
    /// braced argument `expr` resolves `$var` itself, so a string-valued
    /// variable is a valid operand; in the other forms Tcl substitutes the
    /// values textually before parsing, so a non-numeric value becomes an
    /// invalid bareword and the whole command errors — folding that to a
    /// number would turn an erroring program into a value. The non-braced
    /// form therefore binds numeric constants only.
    fn expression_route<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
        &self,
        rest: Option<&str>,
        uses: &HashMap<Symbol, Version, S1>,
        values: &HashMap<ValueKey, LatticeValue, S2>,
        ssa: &SsaFunction,
    ) -> Option<LatticeValue> {
        let arg = rest?.trim();
        let braced = arg.starts_with('{');
        let expr_text = strip_one_level(arg);
        let expr = crate::expr_parser::parse_expr_for_profile(expr_text, self.policy.dialect);
        let env = if braced {
            crate::sccp::env_from_uses(uses, values, ssa)
        } else {
            crate::sccp::env_from_uses_numeric(uses, values, ssa)
        };
        eval_tcl_expr_with_policy(&expr, &env, self.policy)
            .map(|v| LatticeValue::Const(crate::sccp::tcl_value_to_const(v)))
    }
}

static EMPTY_NAMES: BTreeSet<String> = BTreeSet::new();

/// The source-word classification of one operand text: a literal unless
/// it substitutes.
fn word_of(text: &str) -> InvocationWord<'_> {
    if text.contains('$') || text.contains('[') {
        InvocationWord::Dynamic
    } else {
        InvocationWord::Literal(text)
    }
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
        let text = operand.text;
        if let Some(name) = simple_var_ref_name(text) {
            return self.named_fact(name);
        }
        if text.contains('$') || text.contains('[') {
            return FactView::Top(DeclineReason::NotExact);
        }
        FactView::Exact(ExactValue::from_literal(text), None)
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
        let ver = self.uses.get(&sym).copied().unwrap_or(0);
        self.values
            .get(&(sym, ver))
            .map_or(FactView::Pending, |value| {
                lattice_to_fact(value, Some(identity_of((sym, ver))))
            })
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
        &self.driver.context
    }
}

/// The bounded-loop simulator's cell update: `command name ?amount?`
/// resolved through `registry` and evaluated by the registry's declared
/// route over `env`, the simulator's constant environment. The written
/// value replaces the target's entry; a decline leaves `env` untouched and
/// answers `false`, which ends the simulation.
pub(crate) fn exec_cell_update_in_env(
    registry: &CommandRegistry,
    profile: Option<&'static tcl_dialect::DialectProfile>,
    command: &str,
    name: &str,
    amount: Option<&str>,
    env: &mut crate::static_loops::StaticEnv,
) -> bool {
    let texts: Vec<&str> = std::iter::once(name).chain(amount).collect();
    let words: Vec<InvocationWord<'_>> = std::iter::once(InvocationWord::Literal(name))
        .chain(amount.map(word_of))
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
    if let Some(name) = text.strip_prefix("${").and_then(|s| s.strip_suffix('}')) {
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

/// Strip one level of `{…}` or `"…"` wrapping. The inside is kept exactly:
/// `{ a }` is the three-character string ` a `, and a value is a value,
/// not a source token to tidy.
pub(crate) fn strip_one_level(text: &str) -> &str {
    if text.len() >= 2 {
        let bytes = text.as_bytes();
        if (bytes[0] == b'{' && bytes[text.len() - 1] == b'}')
            || (bytes[0] == b'"' && bytes[text.len() - 1] == b'"')
        {
            return &text[1..text.len() - 1];
        }
    }
    text
}

/// Resolve a single command operand to its constant string value: a
/// literal word (optionally brace/quote wrapped), or a pure `$var` /
/// `${var}` whose SCCP lattice value is a constant. Returns `None` for
/// anything that is not a compile-time constant (array refs, command
/// substitutions, unknown vars), so the caller skips folding.
fn resolve_const_string<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
    arg: &str,
    uses: &HashMap<Symbol, Version, S1>,
    values: &HashMap<ValueKey, LatticeValue, S2>,
    ssa: &SsaFunction,
) -> Option<String> {
    let arg = arg.trim();
    if let Some(rest) = arg.strip_prefix('$') {
        // `$name` or `${name}` — reject compound refs (array element,
        // nested substitution, multiple words).
        let name = rest
            .strip_prefix('{')
            .and_then(|r| r.strip_suffix('}'))
            .unwrap_or(rest);
        if name.is_empty()
            || name.contains(|c: char| {
                c.is_whitespace() || c == '(' || c == '[' || c == '$' || c == '"'
            })
        {
            return None;
        }
        return lattice_const_text(name, uses, values, ssa);
    }
    // A literal word with no interpolation or command substitution.
    if !arg.contains('$') && !arg.contains('[') {
        return Some(strip_one_level(arg).to_owned());
    }
    None
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

    #[test]
    fn split_head_basic() {
        assert_eq!(split_head("cmd arg1 arg2"), ("cmd", Some("arg1 arg2")));
        assert_eq!(split_head("  cmd"), ("cmd", None));
        assert_eq!(split_head(""), ("", None));
    }

    #[test]
    fn strip_one_level_braces_and_quotes() {
        assert_eq!(strip_one_level("{abc}"), "abc");
        assert_eq!(strip_one_level("\"abc\""), "abc");
        assert_eq!(strip_one_level("bare"), "bare");
        assert_eq!(strip_one_level("{}"), "");
    }

    /// A value is a value, not a source token: the inside of a braced word
    /// keeps its whitespace.
    #[test]
    fn strip_one_level_keeps_the_inside_exact() {
        assert_eq!(strip_one_level("{ a }"), " a ");
        assert_eq!(strip_one_level("\"\ta\n\""), "\ta\n");
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
}
