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

//! Proc-level transfer summaries (`docs/design/compiler/value-transfers.md`
//! § *Proc-level transfer summaries*): what a call to a procedure of the
//! module does to its caller's places.
//!
//! A procedure's summary comes from its own lattice with its parameters
//! unknown, so it holds for every caller. A parameter whose value names a
//! caller place the body links a local to (`upvar 1 $name v`) has a `Name`
//! role whose outcomes are read from two runs over the body — the place bound
//! on entry, then unbound — at every normal exit; a run that calls another
//! procedure of the module with such a local takes the callee's outcomes
//! there, so the summaries compose bottom-up over the call graph, and a
//! cycle is solved from the procedures never completing, round by round,
//! until nothing changes. A procedure that reaches code the module cannot
//! see, or a frame its summary cannot name, has none: a call to it is a
//! barrier.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::hash::{Hash as _, Hasher as _};
use std::rc::Rc;

use crate::ir::CommandTokens;
use tcl_core_types::RecursionLimit;
use tcl_registry::CommandRegistry;
use tcl_registry::completion::{CompletionCode, CompletionCodeDomain};
use tcl_registry::frame_effect::FrameLevel;
use tcl_registry::value_transfer::{
    BindingIdentity, BindingKind, DependencyEvidence, ExactValue, Existence, ExistenceOutcome,
    ParameterDefault, PlaceRef,
};
use tcl_registry::world_effect::EffectFootprint;
use tcl_syntax::formal_params::{FormalArgumentBinding, FormalParameter};

mod source_context;
pub(crate) use source_context::{OriginalSummaryExpression, OriginalSummaryScript};
use source_context::{ProcedureCall, SourceSummaryContext};

use crate::cfg::{Function as CfgFunction, Terminator};
use crate::cfg_builder::global_write_info::GlobalWriteInfo;
use crate::cfg_builder::upvar_info::{FrameReach, UpvarInfo};
use crate::command_binding::{ModuleCommandMutations, ProcBindingTrustProjection};
use crate::ir::{Statement, SyntheticMarker};
use crate::sccp::{
    BuiltinFoldInputs, CallerPlaces, ExistenceEntry, FoldTrust, ModuleLevel, ModuleRun, SccpResult,
    SolveInputs, TraceInputs,
};
use crate::ssa::SsaFunction;
use crate::value_transfer::ExistenceStep;

use super::ReturnKind;

/// What a call to one procedure does to its caller's places, from the
/// procedure's own analysis: each parameter's role, the places outside its
/// frame it may write, its return shape, how a call may complete, the world
/// effects of what it runs, and the bindings the answer rests on.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TransferSummary {
    /// One role per formal parameter, in declaration order.
    pub(crate) params: Vec<ParamRole>,
    /// The places outside the procedure's frame it may write, each with its
    /// outcome, in order.
    pub(crate) globals: Vec<(PlaceRef, ExistenceOutcome)>,
    /// The return shape the interprocedural analysis derives over the
    /// seedless lattice; a computed result until that analysis states one.
    pub(crate) result: ReturnKind,
    /// The completion codes a call may end with.
    pub(crate) completion: CompletionCodeDomain,
    /// The world effects of the commands the body runs and of its callees.
    pub(crate) effects: EffectFootprint,
    /// The procedure bindings the summary rests on.
    pub(crate) evidence: DependencyEvidence,
    /// Each `Name` parameter's index and the local the body links to the
    /// place it names, by index.
    pub(crate) links: Vec<(usize, String)>,
}

/// Each procedure's transfer summary, by qualified name; a procedure a call
/// to which is a barrier has none.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TransferSummaries(pub(crate) HashMap<String, TransferSummary>);

impl TransferSummaries {
    /// `qname`'s summary as the Explorer prints it, its parameters named
    /// `params`: each parameter's role, the outer places it may write, and
    /// how a call may complete. `None` for a procedure with none, a call to
    /// which is a barrier.
    #[must_use]
    pub fn describe(&self, qname: &str, params: &[String]) -> Option<String> {
        let summary = self.0.get(qname)?;
        let mut parts: Vec<String> = summary
            .params
            .iter()
            .zip(params)
            .map(|(role, param)| match role {
                ParamRole::Value => format!("{param}: value"),
                ParamRole::Unused => format!("{param}: unused"),
                ParamRole::Name { level, outcomes } => {
                    format!(
                        "{param}: names a place {}, {}",
                        level_text(*level),
                        outcomes_text(outcomes)
                    )
                }
            })
            .collect();
        if !summary.globals.is_empty() {
            let globals: Vec<String> = summary
                .globals
                .iter()
                .map(|(place, outcome)| format!("{} {}", place.name, outcomes_text(&[*outcome])))
                .collect();
            parts.push(format!("globals: {}", globals.join(", ")));
        }
        parts.push(format!(
            "completes: {}",
            match summary.completion {
                CompletionCodeDomain::Exact(codes) => codes
                    .iter()
                    .map(|code| match code {
                        CompletionCode::Ok => "ok".to_owned(),
                        CompletionCode::Error => "error".to_owned(),
                        CompletionCode::Return => "return".to_owned(),
                        CompletionCode::Break => "break".to_owned(),
                        CompletionCode::Continue => "continue".to_owned(),
                        CompletionCode::Other(code) => code.to_string(),
                    })
                    .collect::<Vec<_>>()
                    .join(" or "),
                CompletionCodeDomain::Any => "any code".to_owned(),
            }
        ));
        Some(parts.join(" · "))
    }
}

/// A frame level as the Explorer spells it.
fn level_text(level: FrameLevel) -> String {
    match level {
        FrameLevel::Relative(n) => format!("{n} up"),
        FrameLevel::Absolute(n) => format!("at #{n}"),
        FrameLevel::Dynamic => "at a computed level".to_owned(),
    }
}

/// Outcomes in order, as the Explorer spells them.
fn outcomes_text(outcomes: &[ExistenceOutcome]) -> String {
    let kind = |kind: BindingKind| match kind {
        BindingKind::Scalar => "scalar",
        BindingKind::Array => "array",
        BindingKind::Either => "scalar or array",
    };
    outcomes
        .iter()
        .map(|outcome| match outcome {
            ExistenceOutcome::Bind(k) => format!("binds a {}", kind(*k)),
            ExistenceOutcome::Unbind => "unbinds".to_owned(),
            ExistenceOutcome::Preserve => "leaves it".to_owned(),
            ExistenceOutcome::MayBind(k) => format!("may bind a {}", kind(*k)),
        })
        .collect::<Vec<_>>()
        .join(" then ")
}

/// What a procedure does with one argument.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ParamRole {
    /// A value the body reads.
    Value,
    /// The name of a place in the frame `level` selects, which the body
    /// links a local to and applies `outcomes` to, in order.
    Name {
        /// The frame the place is in.
        level: FrameLevel,
        /// What the body does to the place's existence.
        outcomes: Vec<ExistenceOutcome>,
    },
    /// Never read.
    Unused,
}

/// What a call does to the places its `Name` arguments name
/// ([`ModuleProcedures::call_transfer`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CallTransfer {
    /// The call never completes normally: the callee's parameters reject
    /// its word count, or the callee never completes normally.
    Never,
    /// Each place, with the step the callee's outcomes on it compose to.
    Places(Vec<(String, ExistenceStep)>),
}

/// The completion codes of a procedure whose body runs no `return` with
/// options: a `break` or `continue` that leaves the body is an error.
const NORMAL_OR_ERROR: &[CompletionCode] = &[CompletionCode::Ok, CompletionCode::Error];

/// The completion codes of a procedure that never completes normally.
const ERROR_ONLY: &[CompletionCode] = &[CompletionCode::Error];

/// The outcomes that leave a place may-bound whatever it held: the answer
/// for a place the analysis cannot follow.
const ANY_OUTCOME: [ExistenceOutcome; 2] = [
    ExistenceOutcome::Unbind,
    ExistenceOutcome::MayBind(BindingKind::Either),
];

/// What a module's summaries are computed from.
#[derive(Clone, Copy)]
pub(crate) struct ModuleInputs<'a> {
    /// The lowered module.
    pub(crate) ir: &'a crate::ir::Module,
    /// Its flow graphs.
    pub(crate) cfg: &'a crate::cfg::CfgModule,
    /// Each procedure's caller-frame effects, keyed by every spelling.
    pub(crate) frames: &'a HashMap<String, UpvarInfo>,
    /// Each procedure's writes to global and namespace places.
    pub(crate) outer_writes: &'a HashMap<String, GlobalWriteInfo>,
    /// The registry the module resolved against.
    pub(crate) registry: &'a CommandRegistry,
    /// The module's command mutations.
    pub(crate) mutations: &'a ModuleCommandMutations,
    /// The procedure-binding trust the shared lattice reads.
    pub(crate) projection: &'a ProcBindingTrustProjection,
    /// The module's variable traces.
    pub(crate) trace: crate::compilation_unit::ModuleTraceFacts<'a>,
    /// The document's lexer configuration.
    pub(crate) config: tcl_lexer::LexerConfig,
    /// The analysis context the module's lattices run under.
    pub(crate) analysis_context: &'a crate::value_transfer::AnalysisContextKey,
}

/// The module's procedures as a lattice run reads them: their parameters
/// and defaults, whether a name still denotes one, and their transfer
/// summaries.
pub(crate) struct ModuleProcedures<'a> {
    /// Actual lowered source owner for fresh CFGs and procedure reruns.
    ir: &'a crate::ir::Module,
    procedures: &'a HashMap<String, crate::ir::Procedure>,
    redefined: &'a HashSet<String>,
    mutations: &'a ModuleCommandMutations,
    projection: &'a ProcBindingTrustProjection,
    source: Option<SourceSummaryContext<'a>>,
    /// The finished summaries, by qualified name.
    summaries: RefCell<HashMap<String, TransferSummary>>,
    /// The roles of the procedures of a cycle being solved: `None` while a
    /// procedure is not yet known to complete normally.
    provisional: RefCell<HashMap<String, Option<Vec<ParamRole>>>>,
    /// The revision every summary rides on.
    revision: u64,
    /// The module's flow graphs.
    cfg: &'a crate::cfg::CfgModule,
    /// The registry the module resolved against.
    registry: &'a CommandRegistry,
    /// The document's lexer configuration.
    config: tcl_lexer::LexerConfig,
    /// The module's variable traces.
    trace: crate::compilation_unit::ModuleTraceFacts<'a>,
    /// The analysis context the module's lattices run under, where the run
    /// that reads the procedures has one.
    analysis_context: Option<&'a crate::value_transfer::AnalysisContextKey>,
    /// The built unit's procedures, once there is a unit: their flow graphs
    /// and SSA are what a re-run solves.
    units: Option<&'a HashMap<String, crate::compilation_unit::FunctionUnit>>,
    /// Each procedure's SSA, built or borrowed once for its re-runs.
    ssa: RefCell<HashMap<String, Rc<SsaFunction>>>,
    /// Each re-run made: by callee, seeds and whole-module trust.
    reruns: RefCell<HashMap<RerunKey, Option<Rc<Rerun>>>>,
    /// The re-runs under way, innermost last.
    active: RefCell<Vec<RerunKey>>,
    /// How many re-runs were made.
    spent: Cell<u32>,
}

/// A re-run's identity: the callee, the bytes each parameter and place is
/// seeded with and each place's existence, and whether the run takes the
/// whole-module trust. The fold policy and the dialect a re-run is solved
/// under are not in it: both are fixed for one [`ModuleProcedures`] — one
/// per unit build, one per seedless stage and one per O103 fold — so every
/// re-run it holds shares them.
type RerunKey = (
    String,
    Vec<Option<Vec<u8>>>,
    Vec<(Option<Vec<u8>>, Existence)>,
    bool,
);

/// What one call gives a re-run of its callee: each parameter's value, where
/// exact, and each `Name` link's place — its value, where exact, and its
/// existence — in [`TransferSummary::links`] order.
pub(crate) type RerunSeeds<'s> = (
    &'s [Option<ExactValue>],
    &'s [(Option<ExactValue>, Existence)],
);

/// How deep re-runs nest: a call a re-run evaluates re-runs its callee in
/// turn.
const MAX_RERUN_DEPTH: usize = 32;

/// How many re-runs one module's procedures make in all.
const MAX_RERUNS: u32 = 4096;

/// A re-run of a procedure's body under what one call gives it
/// ([`ModuleProcedures::rerun`]).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Rerun {
    /// The value every normal exit returns, where they agree.
    pub(crate) result: Option<ExactValue>,
    /// Whether a normal exit is reached: where none is, the call never
    /// completes normally.
    pub(crate) completes: bool,
    /// Whether the run decided that a call so seeded completes at those
    /// exits: every parameter and every place it was seeded with exact, no
    /// statement raising beside them and every call it applied decided in
    /// turn ([`crate::sccp::RunCompletion::is_decided`]).
    pub(crate) decided: bool,
    /// Each `Name` link's place where the normal exits leave it, in
    /// [`TransferSummary::links`] order.
    pub(crate) places: Vec<PlaceAfter>,
}

/// One caller place where a re-run's normal exits leave it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PlaceAfter {
    /// Its existence, joined over the exits; `None` where no exit states
    /// it.
    pub(crate) existence: Option<Existence>,
    /// The value every exit leaves in it, where they agree.
    pub(crate) value: Option<ExactValue>,
}

/// What a re-run is solved under: its caller's value semantics, and the
/// registry, mutation facts and trust a fold is gated by.
#[derive(Clone, Copy)]
pub(crate) struct RerunStance<'s> {
    /// The value semantics.
    pub(crate) policy: crate::tcl_expr_eval::FoldPolicy,
    /// The fold inputs.
    pub(crate) folds: BuiltinFoldInputs<'s>,
}

impl<'a> ModuleProcedures<'a> {
    /// The module's procedures with their summaries computed: a procedure
    /// whose summary needs a lattice run only at the deep tier, where its
    /// existence rung runs.
    pub(crate) fn new(inputs: ModuleInputs<'a>) -> Self {
        Self::with_rounds(inputs, super::MAX_INTERPROCEDURAL_WALK_DEPTH)
    }

    /// [`Self::new`] with a cycle given at most `rounds` rounds to settle.
    pub(crate) fn with_rounds(inputs: ModuleInputs<'a>, rounds: RecursionLimit) -> Self {
        let procedures = Self {
            ir: inputs.ir,
            procedures: &inputs.ir.procedures,
            redefined: &inputs.ir.redefined_procedures,
            mutations: inputs.mutations,
            projection: inputs.projection,
            source: SourceSummaryContext::for_module(inputs.ir, inputs.registry, inputs.config),
            summaries: RefCell::new(HashMap::new()),
            provisional: RefCell::new(HashMap::new()),
            revision: revision_of(inputs.ir, inputs.mutations),
            cfg: inputs.cfg,
            registry: inputs.registry,
            config: inputs.config,
            trace: inputs.trace,
            analysis_context: Some(inputs.analysis_context),
            units: None,
            ssa: RefCell::new(HashMap::new()),
            reruns: RefCell::new(HashMap::new()),
            active: RefCell::new(Vec::new()),
            spent: Cell::new(0),
        };
        if inputs.analysis_context.tier == tcl_registry::value_transfer::AnalysisTier::Deep {
            procedures.summarise(&inputs, rounds);
        }
        procedures
    }

    /// The procedures of a built unit, with the summaries its build
    /// computed, for a run made after the build: a seedless return run and
    /// O103's argument-sensitive re-run.
    pub(crate) fn of_unit(
        cu: &'a crate::compilation_unit::CompilationUnit,
        registry: &'a CommandRegistry,
    ) -> Self {
        Self {
            ir: &cu.ir_module,
            procedures: &cu.ir_module.procedures,
            redefined: &cu.ir_module.redefined_procedures,
            mutations: &cu.command_mutations,
            projection: &cu.caller_scope.proc_binding_trust,
            source: SourceSummaryContext::for_module(
                &cu.ir_module,
                registry,
                cu.ir_module.lexer_config,
            ),
            summaries: RefCell::new(cu.transfers.0.clone()),
            provisional: RefCell::new(HashMap::new()),
            revision: revision_of(&cu.ir_module, &cu.command_mutations),
            cfg: &cu.cfg_module,
            registry,
            config: cu.ir_module.lexer_config,
            trace: crate::compilation_unit::ModuleTraceFacts::of(&cu.ir_module),
            analysis_context: None,
            units: Some(&cu.procedures),
            ssa: RefCell::new(HashMap::new()),
            reruns: RefCell::new(HashMap::new()),
            active: RefCell::new(Vec::new()),
            spent: Cell::new(0),
        }
    }

    /// The revision the summaries ride on: the module's procedure bodies,
    /// parameters and redefinitions, and its command bindings, so a
    /// `rename` or a redefinition anywhere in the module changes it.
    pub(crate) const fn revision(&self) -> u64 {
        self.revision
    }

    /// The finished summaries.
    pub(crate) fn into_summaries(self) -> TransferSummaries {
        TransferSummaries(self.summaries.into_inner())
    }

    /// The procedure of the module `head` names from `function`, where its
    /// binding stands under `trust`: no redefinition, and no `rename` or
    /// alias that moves the name ([`FoldTrust::WholeModule`] also asks that
    /// no binding in the module is computed).
    fn resolve_name(&self, head: &str, function: &str, trust: FoldTrust) -> Option<String> {
        let source = self.source.as_ref()?;
        let qname = super::resolve_internal_call_with(head, function, |qname| {
            source.procedure(qname).is_some()
        })?;
        let stands = !self.redefined.contains(&qname)
            && match trust {
                FoldTrust::WholeModule => self.mutations.trusts_proc_binding(&qname),
                FoldTrust::ObservedBindings => {
                    self.projection.trusts_proc_binding(&qname)
                        && self.mutations.observed_proc_binding(&qname)
                }
            };
        stands.then_some(qname)
    }

    /// A procedure summary call requires the same retained source point,
    /// original declaration and effective argv. A reporting head is no join.
    pub(crate) fn original_call_arguments(
        &self,
        call: &ProcedureCall<'_>,
    ) -> Option<Vec<Option<String>>> {
        call.literal_arguments(self.ir)
    }

    pub(crate) fn original_statement_tokens(
        &self,
        qname: &str,
        statement: &Statement,
    ) -> Option<CommandTokens> {
        self.source.as_ref()?.statement_tokens(qname, statement)
    }

    pub(crate) fn original_expression(
        &self,
        parent: &CommandTokens,
        expression: &crate::expr_ast::ExprNode,
        base: Option<u32>,
    ) -> Option<OriginalSummaryExpression> {
        self.source.as_ref()?.expression(parent, expression, base)
    }

    pub(crate) fn original_expression_substitution(
        &self,
        expression: &OriginalSummaryExpression,
        script: &str,
        start: u32,
        end: u32,
    ) -> Option<OriginalSummaryScript> {
        self.source
            .as_ref()?
            .expression_substitution(expression, script, start, end)
    }

    pub(crate) fn original_substitution(
        &self,
        parent: &CommandTokens,
        site: &crate::ir::SourceSite,
        script: &str,
    ) -> Option<OriginalSummaryScript> {
        self.source.as_ref()?.substitution(parent, site, script)
    }

    pub(crate) fn original_body(
        &self,
        parent: &CommandTokens,
        argument: usize,
        script: &str,
    ) -> Option<OriginalSummaryScript> {
        self.source.as_ref()?.body(parent, argument, script)
    }

    pub(crate) fn resolve_call_at(
        &self,
        tokens: &CommandTokens,
        trust: FoldTrust,
    ) -> Option<ProcedureCall<'a>> {
        let call = self.source.as_ref()?.procedure_call(tokens)?;
        let name = &call.procedure.qualified_name;
        let stands = !self.redefined.contains(name)
            && match trust {
                FoldTrust::WholeModule => self.mutations.trusts_proc_binding(name),
                FoldTrust::ObservedBindings => {
                    self.projection.trusts_proc_binding(name)
                        && self.mutations.observed_proc_binding(name)
                }
            };
        stands.then_some(call)
    }

    /// Borrow the selected original operation and its effective source argv
    /// under this Module's complete point metadata. This is conditional value
    /// modelling, independently of Native execution and observer permissions.
    pub(crate) fn with_original_operation<T>(
        &self,
        tokens: &CommandTokens,
        apply: impl FnOnce(
            &crate::registry_invocation::ResolvedStatementInvocation,
            &tcl_registry::ResolvedInvocation<'_, '_>,
        ) -> Option<T>,
    ) -> Option<T> {
        self.source.as_ref()?.with_operation(tokens, apply)
    }

    /// The default `info default` reads for `procedure`'s `parameter`, the
    /// procedure named from `function`: its value, none, or unknown where
    /// the name denotes no procedure of the module whose binding stands, or
    /// the procedure has no such parameter.
    pub(crate) fn parameter_default(
        &self,
        function: &str,
        (procedure, parameter): (&str, &str),
        trust: FoldTrust,
    ) -> ParameterDefault {
        let Some(declared) = self
            .resolve_name(procedure, function, trust)
            .and_then(|qname| self.procedures.get(&qname))
        else {
            return ParameterDefault::Unknown;
        };
        let Some((formals, _)) = self.formals(&declared.qualified_name) else {
            return ParameterDefault::Unknown;
        };
        match formals.into_iter().find(|formal| formal.name == parameter) {
            Some(formal) => formal.default.map_or(ParameterDefault::None, |value| {
                ParameterDefault::Value(ExactValue::from_literal(&value))
            }),
            None => ParameterDefault::Unknown,
        }
    }

    /// What a call to `callee` with these argument words does to the places
    /// its `Name` arguments name: `words` holds each argument's literal
    /// value, `None` for one that substitutes. A `Name` parameter the call
    /// omits names the place its default spells, as Tcl binds the default.
    /// The places come in [`TransferSummary::links`] order. A word count
    /// the callee's parameters reject is a call that never completes. `None`
    /// when the call is a barrier — a callee with no summary or parameters
    /// the analysis cannot read, or a `Name` argument that substitutes,
    /// names an element or names the place another one does.
    pub(crate) fn call_transfer(
        &self,
        callee: &str,
        words: &[Option<&str>],
    ) -> Option<CallTransfer> {
        let (formals, grammar) = self.formals(callee)?;
        let shape = tcl_syntax::formal_params::formal_argument_count_shape(
            formals
                .iter()
                .map(|formal| (formal.name == "args", formal.default.is_some())),
            grammar,
        );
        if !shape.accepts(words.len()) {
            return Some(CallTransfer::Never);
        }
        let bindings =
            tcl_syntax::formal_params::bind_formal_arguments(&formals, words.len(), grammar)
                .ok()?;
        let roles = match self.provisional.borrow().get(callee) {
            Some(None) => return Some(CallTransfer::Never),
            Some(Some(roles)) => roles.clone(),
            None => {
                let summaries = self.summaries.borrow();
                let summary = summaries.get(callee)?;
                if summary.completion == CompletionCodeDomain::Exact(ERROR_ONLY) {
                    return Some(CallTransfer::Never);
                }
                summary.params.clone()
            }
        };
        let mut places: Vec<(String, ExistenceStep)> = Vec::new();
        for (index, role) in roles.iter().enumerate() {
            let ParamRole::Name { level, outcomes } = role else {
                continue;
            };
            if *level != FrameLevel::Relative(1) {
                return None;
            }
            let binding = bindings.iter().find(|binding| match binding {
                FormalArgumentBinding::Value { parameter, .. }
                | FormalArgumentBinding::Default { parameter }
                | FormalArgumentBinding::Rest { parameter, .. }
                | FormalArgumentBinding::CallerLink { parameter, .. } => *parameter == index,
            })?;
            let word = match binding {
                FormalArgumentBinding::Value { argument, .. } => (*words.get(*argument)?)?,
                FormalArgumentBinding::Default { parameter } => {
                    formals.get(*parameter)?.default.as_deref()?
                }
                FormalArgumentBinding::Rest { .. } | FormalArgumentBinding::CallerLink { .. } => {
                    return None;
                }
            };
            if word.is_empty()
                || tcl_syntax::naming::split_element_ref_bytes(word.as_bytes()).is_some()
                || places.iter().any(|(held, _)| held == word)
            {
                return None;
            }
            places.push((word.to_owned(), step_of(outcomes)));
        }
        Some(CallTransfer::Places(places))
    }

    /// The existence step a call to `callee` takes on each outer place it
    /// may write, by the place's name.
    pub(crate) fn outer_steps(&self, callee: &str) -> Vec<(String, ExistenceStep)> {
        let summaries = self.summaries.borrow();
        let Some(summary) = summaries.get(callee) else {
            return Vec::new();
        };
        let mut steps: Vec<(String, ExistenceStep)> = Vec::new();
        for (place, outcome) in &summary.globals {
            let step = ExistenceStep::of(*outcome);
            match steps.iter_mut().find(|(name, _)| *name == place.name) {
                Some((_, held)) => *held = held.then(step),
                None => steps.push((place.name.clone(), step)),
            }
        }
        steps
    }

    /// Whether a call to `callee` reaches no place outside its frame: it has
    /// a summary, names no place and writes no outer one.
    #[cfg(test)]
    fn keeps_to_its_frame(&self, callee: &str) -> bool {
        self.summaries
            .borrow()
            .get(callee)
            .is_some_and(|summary| summary.links.is_empty() && summary.globals.is_empty())
    }

    /// The parameters of the procedure `qname` names, where it is one of
    /// the module's.
    pub(crate) fn params_of(&self, qname: &str) -> Option<&'a [String]> {
        self.source
            .as_ref()?
            .procedure(qname)
            .map(|declared| declared.params.as_slice())
    }

    /// `callee`'s `Name` links: each such parameter's index and the local
    /// its body links to the place it names.
    pub(crate) fn links(&self, callee: &str) -> Option<Vec<(usize, String)>> {
        Some(self.summaries.borrow().get(callee)?.links.clone())
    }

    /// The values a call with `arguments` gives `callee`'s parameters —
    /// each argument's where the call supplies it, else the parameter's
    /// default, and the rest of the arguments as one list for a trailing
    /// `args` — each `None` where it is not exact; `None` where the
    /// parameters do not accept the call.
    pub(crate) fn parameter_values(
        &self,
        callee: &str,
        arguments: &[Option<ExactValue>],
    ) -> Option<Vec<Option<ExactValue>>> {
        let (parameters, grammar) = self.formals(callee)?;
        let bindings =
            tcl_syntax::formal_params::bind_formal_arguments(&parameters, arguments.len(), grammar)
                .ok()?;
        let mut values = vec![None; parameters.len()];
        for binding in bindings {
            let (parameter, value) = match binding {
                FormalArgumentBinding::Value {
                    parameter,
                    argument,
                } => (parameter, arguments.get(argument)?.clone()),
                FormalArgumentBinding::Default { parameter } => (
                    parameter,
                    parameters
                        .get(parameter)?
                        .default
                        .as_deref()
                        .map(ExactValue::from_literal),
                ),
                FormalArgumentBinding::Rest {
                    parameter,
                    start,
                    len,
                    ..
                } => {
                    let rest: Option<Vec<&str>> = arguments
                        .get(start..start.checked_add(len)?)?
                        .iter()
                        .map(|argument| argument.as_ref()?.as_str().ok())
                        .collect();
                    (
                        parameter,
                        rest.and_then(|rest| {
                            tcl_registry::value_transfer::TargetSemantics::of(Some(
                                self.ir.source_metadata_input.as_ref()?.unit_profile(),
                            ))
                            .render_list(&rest)
                            .map(|text| ExactValue::from_literal(&text))
                        }),
                    )
                }
                // A Jim caller link is a cell binding, not a copied scalar value.
                FormalArgumentBinding::CallerLink { .. } => return None,
            };
            *values.get_mut(parameter)? = value;
        }
        Some(values)
    }

    /// A re-run of `callee`'s body with its parameters holding `params` and
    /// the places its `Name` parameters name holding `places`, in
    /// [`TransferSummary::links`] order, under `stance`: its result,
    /// whether it completes normally, and where it leaves each place. Each
    /// re-run is made once. `None` for a callee with no summary or no flow
    /// graph to solve, one whose re-run with these seeds is already under
    /// way — a recursion its seeds do not end — and past the depth and
    /// count bounds.
    pub(crate) fn rerun(
        &self,
        callee: &str,
        (params, places): RerunSeeds<'_>,
        stance: RerunStance<'_>,
    ) -> Option<Rc<Rerun>> {
        let bytes = |value: &Option<ExactValue>| value.as_ref().map(|value| value.bytes.clone());
        let key: RerunKey = (
            callee.to_owned(),
            params.iter().map(bytes).collect(),
            places
                .iter()
                .map(|(value, existence)| (bytes(value), *existence))
                .collect(),
            stance.folds.trust == FoldTrust::WholeModule,
        );
        if let Some(made) = self.reruns.borrow().get(&key) {
            return made.clone();
        }
        if self.active.borrow().contains(&key)
            || self.active.borrow().len() >= MAX_RERUN_DEPTH
            || self.spent.get() >= MAX_RERUNS
        {
            return None;
        }
        self.spent.set(self.spent.get() + 1);
        self.active.borrow_mut().push(key.clone());
        let made = self
            .solve_rerun(callee, (params, places), stance)
            .map(Rc::new);
        self.active.borrow_mut().pop();
        self.reruns.borrow_mut().insert(key, made.clone());
        made
    }

    /// [`Self::rerun`]'s run, uncached.
    fn solve_rerun(
        &self,
        callee: &str,
        (params, places): RerunSeeds<'_>,
        stance: RerunStance<'_>,
    ) -> Option<Rerun> {
        let declared = self.source.as_ref()?.procedure(callee)?;
        let links = self.links(callee)?;
        let (cfg, ssa) = self.flow_of(callee)?;
        let mut seed: HashMap<(String, crate::ssa::Version), crate::analyses::LatticeValue> =
            HashMap::new();
        for (name, value) in declared.params.iter().zip(params) {
            if let Some(value) = value {
                seed.insert(
                    (name.clone(), 0),
                    crate::value_transfer::exact_to_lattice(value),
                );
            }
        }
        let mut entries: HashMap<String, Existence> = HashMap::new();
        for ((_, local), (value, existence)) in links.iter().zip(places) {
            if let Some(value) = value {
                seed.insert(
                    (local.clone(), 0),
                    crate::value_transfer::exact_to_lattice(value),
                );
            }
            entries.insert(local.clone(), *existence);
        }
        let owned: HashSet<String> = entries.keys().cloned().collect();
        let caller_places = CallerPlaces { entries };
        let trace = self.trace;
        let source_metadata_input = self.source_metadata_input_for(callee);
        if source_metadata_input.is_none() && !cfg.metadata_context.is_standalone() {
            return None;
        }
        let config = self.units.and_then(|units| units.get(callee)).map_or(
            self.config,
            crate::compilation_unit::FunctionUnit::source_lexer_config,
        );
        let callee_folds = BuiltinFoldInputs {
            source_metadata_input,
            dialect: source_metadata_input
                .map(crate::analyser::ResolvedAnalysisInput::unit_profile)
                .or_else(|| {
                    cfg.metadata_context
                        .is_standalone()
                        .then_some(stance.folds.dialect)
                        .flatten()
                }),
            proven_pure_parameters: false,
            ..stance.folds
        };
        let result = crate::sccp::sccp_in_module(&SolveInputs {
            cfg,
            ssa: &ssa,
            param_constants: Some(&seed),
            policy: stance.policy,
            extra_escaping: &HashSet::new(),
            trace: TraceInputs {
                source_metadata_input,
                registry: self.registry,
                traced_variables: trace.traced_variables,
                has_dynamic_variable_trace: trace.has_dynamic_variable_trace,
                deferred_writes: trace.deferred_writes,
                analysis_context: self.analysis_context,
                existence: Some(ExistenceEntry {
                    params: &declared.params,
                    object_state: None,
                    initial_global: false,
                    connection_scoped: None,
                    dynamic_trace: trace.has_dynamic_variable_trace || trace.deferred_writes.any,
                    config,
                    caller_places: Some(&caller_places),
                }),
            },
            folds: Some(callee_folds),
            module: ModuleRun {
                procedures: Some(self),
                owned: Some(&owned),
                level: ModuleLevel::Results,
                reads_exits: true,
            },
        });
        let exits = normal_exits(cfg, &result);
        let seeded_exactly = params.iter().all(Option::is_some)
            && places.iter().all(|(value, existence)| match existence {
                Existence::Unbound => true,
                Existence::Bound(_) => value.is_some(),
                Existence::Pending | Existence::MayBound => false,
            });
        let reading = super::ExitReading {
            policy: stance.policy,
            grammar: config.grammar_over(
                callee_folds
                    .dialect
                    .map_or_else(tcl_dialect::LexerGrammar::default, |profile| {
                        profile.grammar
                    }),
            ),
            folds: callee_folds,
            module: Some((self, callee)),
        };
        Some(Rerun {
            result: super::exit_value(super::ExitBody { cfg, ssa: &ssa }, &result, reading),
            completes: !exits.is_empty(),
            decided: seeded_exactly && result.completion.is_decided(),
            places: links
                .iter()
                .map(|(_, local)| PlaceAfter {
                    existence: exit_fact(&ssa, &result, &exits, local),
                    value: exit_constant(cfg, &ssa, &result, &exits, local),
                })
                .collect(),
        })
    }

    /// Reborrow this function's complete input only under the same Module owner.
    pub(crate) fn invocation_metadata_context_for_function<'f>(
        &self,
        function: &'f crate::compilation_unit::FunctionUnit,
        registry: &CommandRegistry,
    ) -> Option<crate::registry_invocation::InvocationMetadataContext<'f>> {
        function.invocation_metadata_context_for_module(registry, self.ir)
    }

    /// Metadata for an exact original function point under this Module owner.
    /// Availability, whole source and original vector correspondence remain
    /// distinct from a successful command or physical variable operation.
    pub(crate) fn original_invocation_metadata_for_function<'f>(
        &self,
        function: &crate::compilation_unit::FunctionUnit,
        tokens: &'f crate::ir::CommandTokens,
        registry: &CommandRegistry,
    ) -> Option<crate::registry_invocation::InvocationMetadataContext<'f>> {
        self.invocation_metadata_context_for_function(function, registry)?;
        tokens
            .source_binding
            .as_ref()?
            .original_invocation_metadata_for_module(tokens, self.ir, registry)
    }

    /// A built callee keeps its own source input. Its missing input is not
    /// repaired from a sibling module record; fresh CFGs use the actual module.
    fn source_metadata_input_for(
        &self,
        callee: &str,
    ) -> Option<&'a crate::analyser::ResolvedAnalysisInput> {
        match self.units {
            Some(units) => units
                .get(callee)?
                .invocation_metadata_context_for_module(self.registry, self.ir)?
                .source_analysis_input(),
            None => crate::registry_invocation::InvocationMetadataContext::for_module(
                self.registry,
                self.ir,
            )?
            .source_analysis_input(),
        }
    }

    /// `callee`'s flow graph and SSA: the built unit's where there is one,
    /// else the module's flow graph with its SSA built once. `None` for a
    /// body the complexity guard stops.
    fn flow_of(&self, callee: &str) -> Option<(&'a CfgFunction, Rc<SsaFunction>)> {
        let declared = self.procedures.get(callee)?;
        let unit = self.units.and_then(|units| units.get(callee));
        let cfg = match unit {
            Some(unit) => &unit.cfg,
            None => self.cfg.procedures.get(callee)?,
        };
        if crate::ssa::is_complexity_guarded(cfg)
            || unit.is_some_and(|unit| unit.complexity_guarded)
        {
            return None;
        }
        let ssa = self
            .ssa
            .borrow_mut()
            .entry(callee.to_owned())
            .or_insert_with(|| {
                Rc::new(unit.map_or_else(
                    || {
                        crate::ssa::build_ssa_with_context_for_entry_and_metadata(
                            cfg,
                            self.registry,
                            self.config,
                            crate::compilation_unit::function_source_entry(cfg, None),
                            Some(&declared.params),
                            crate::registry_invocation::InvocationMetadataContext::for_module(
                                self.registry,
                                self.ir,
                            ),
                        )
                    },
                    |unit| unit.ssa.clone(),
                ))
            })
            .clone();
        Some((cfg, ssa))
    }

    /// `callee`'s formal parameters, each with the default Tcl binds an
    /// omitted argument to.
    fn formals(
        &self,
        callee: &str,
    ) -> Option<(Vec<FormalParameter>, tcl_dialect::ParameterGrammar)> {
        let declared = self.source.as_ref()?.procedure(callee)?;
        crate::registry_invocation::original_procedure_parameters(self.ir, declared, self.registry)
    }

    /// Compute every procedure's summary, callees before callers.
    fn summarise(&self, inputs: &ModuleInputs<'_>, rounds: RecursionLimit) {
        // A module that may rebind a builtin may turn a typed statement into
        // a call this scan cannot see.
        if self.source.is_none() || inputs.mutations.rebinds_builtins() {
            return;
        }
        let mut names: Vec<&str> = inputs.cfg.procedures.keys().map(String::as_str).collect();
        names.sort_unstable();
        let mut calls: HashMap<&str, BTreeSet<String>> = names
            .iter()
            .filter_map(|&qname| Some((qname, self.local_calls(qname, inputs)?)))
            .collect();
        // A procedure that calls a barrier is one.
        loop {
            let barred: Vec<&str> = calls
                .iter()
                .filter(|(_, callees)| {
                    callees
                        .iter()
                        .any(|callee| !calls.contains_key(callee.as_str()))
                })
                .map(|(&qname, _)| qname)
                .collect();
            if barred.is_empty() {
                break;
            }
            for qname in barred {
                calls.remove(qname);
            }
        }
        for component in components(&names, &calls) {
            self.summarise_component(&component, &calls, inputs, rounds);
        }
    }

    /// The procedures of the module `qname` calls, from its statements, the
    /// commands their words substitute and its terminators' words; `None`
    /// when it reaches code or a frame the module cannot see, names a
    /// variable it computes, or is too large to analyse.
    fn local_calls(&self, qname: &str, inputs: &ModuleInputs<'_>) -> Option<BTreeSet<String>> {
        let cfg = inputs.cfg.procedures.get(qname)?;
        let source = self.source.as_ref()?;
        let declared = source.procedure(qname)?;
        // Count/formal syntax is not an activation. This conditional scalar
        // model also needs the existing selected binder's local scalar closure.
        crate::registry_invocation::original_procedure_scalar_bindings(
            self.ir,
            declared,
            self.registry,
        )?;
        let body_bytes = declared.span.end().saturating_sub(declared.span.start()) as usize;
        if body_bytes > crate::ssa::DEEP_ANALYSIS_BODY_BYTES
            || crate::ssa::is_complexity_guarded(cfg)
            || inputs
                .frames
                .get(qname)
                .is_some_and(|frame| !frame_is_closed(frame, &declared.params))
            || inputs
                .outer_writes
                .get(qname)
                .is_some_and(|writes| writes.opaque_global_frame)
        {
            return None;
        }
        let computed =
            crate::dynamic_names::dynamic_name_barrier(cfg, inputs.registry, inputs.config);
        if computed.writes || computed.destroys {
            return None;
        }
        let mut callees = BTreeSet::new();
        for block in cfg.blocks.values() {
            for statement in &block.statements {
                match statement {
                    Statement::Call { tokens, .. }
                        if tokens
                            .as_ref()
                            .and_then(|tokens| tokens.synthetic)
                            .is_some() =>
                    {
                        if matches!(
                            statement.synthetic_marker(),
                            Some(
                                SyntheticMarker::UnseenCall
                                    | SyntheticMarker::GlobalFrameScript
                                    | SyntheticMarker::CallerFrameOpaque
                                    | SyntheticMarker::ArmWrites
                            )
                        ) {
                            return None;
                        }
                    }
                    Statement::Call { .. }
                    | Statement::AssignConst { .. }
                    | Statement::AssignExpr { .. }
                    | Statement::AssignValue { .. }
                    | Statement::Incr { .. }
                    | Statement::ExprEval { .. }
                    | Statement::Return { .. } => {
                        let original = source.statement_tokens(qname, statement)?;
                        if crate::ir_helpers::evaluated_command_substitutions_with_metadata_context(
                            statement,
                            self.registry,
                            Some(source.metadata(&original)?),
                            self.config,
                        )
                        .opaque
                        {
                            return None;
                        }
                    }
                    _ => return None,
                }
            }
        }
        for tokens in source.body_commands(qname)? {
            if aliases_unnamed_place(&tokens, source)? {
                return None;
            }
            self.callee_at(&tokens, &mut callees)?;
        }
        self.outer_arguments(qname, inputs)?;
        Some(callees)
    }

    /// The `Name` arguments of `qname`'s calls that name a place outside its
    /// frame ([`OuterArgument`]). `None` where one names a place the
    /// summary cannot name — a local linked within the frame, to an object's
    /// or a connection's variable, or to more than one place — so the
    /// procedure has no summary.
    fn outer_arguments(
        &self,
        qname: &str,
        inputs: &ModuleInputs<'_>,
    ) -> Option<Vec<OuterArgument>> {
        let cfg = inputs.cfg.procedures.get(qname)?;
        let (namespace, _) = tcl_syntax::naming::key_holder_and_tail(qname);
        let source = self.source.as_ref()?;
        let linked = linked_locals(cfg, namespace, qname, source)?;
        let mut found = Vec::new();
        let mut blocks: Vec<_> = cfg.blocks.iter().collect();
        blocks.sort_unstable_by_key(|(id, _)| **id);
        for (&id, block) in blocks {
            for (index, statement) in block.statements.iter().enumerate() {
                if statement.synthetic_marker().is_some() {
                    continue;
                }
                let parent = source.statement_tokens(qname, statement)?;
                let site = (id == cfg.entry).then_some((id, index));
                self.outer_call_arguments(
                    OuterCall {
                        tokens: &parent,
                        site,
                    },
                    (&linked, namespace),
                    inputs,
                    &mut found,
                )?;
                for child in source.children(&parent)? {
                    self.outer_call_arguments(
                        OuterCall {
                            tokens: &child,
                            site: None,
                        },
                        (&linked, namespace),
                        inputs,
                        &mut found,
                    )?;
                }
            }
        }
        Some(found)
    }

    /// [`Self::outer_arguments`] for one genuine original call.
    fn outer_call_arguments(
        &self,
        call: OuterCall<'_>,
        (linked, namespace): (&HashMap<String, LinkedLocal>, &str),
        inputs: &ModuleInputs<'_>,
        found: &mut Vec<OuterArgument>,
    ) -> Option<()> {
        let source = self.source.as_ref()?;
        let Some(selected) = source.procedure_call(call.tokens) else {
            source.operation(call.tokens)?;
            return Some(());
        };
        let callee = &selected.procedure.qualified_name;
        let declared = source.procedure(callee)?;
        let words = selected.literal_arguments(inputs.ir)?;
        let (formals, grammar) = self.formals(callee)?;
        let shape = tcl_syntax::formal_params::formal_argument_count_shape(
            formals
                .iter()
                .map(|formal| (formal.name == "args", formal.default.is_some())),
            grammar,
        );
        if !shape.accepts(words.len()) {
            return Some(());
        }
        let bindings =
            tcl_syntax::formal_params::bind_formal_arguments(&formals, words.len(), grammar)
                .ok()?;
        for (param, _) in links_of(declared, inputs) {
            let binding = bindings.iter().find(|binding| match binding {
                FormalArgumentBinding::Value { parameter, .. }
                | FormalArgumentBinding::Default { parameter }
                | FormalArgumentBinding::Rest { parameter, .. }
                | FormalArgumentBinding::CallerLink { parameter, .. } => *parameter == param,
            })?;
            let word = match binding {
                FormalArgumentBinding::Value { argument, .. } => {
                    match words.get(*argument)?.as_deref() {
                        Some(value) => value,
                        // The caller's lattice handles a computed local name; it cannot
                        // contribute a particular outer-place claim here.
                        None => continue,
                    }
                }
                FormalArgumentBinding::Default { parameter } => {
                    formals.get(*parameter)?.default.as_deref()?
                }
                FormalArgumentBinding::Rest { .. } | FormalArgumentBinding::CallerLink { .. } => {
                    return None;
                }
            };
            let (places, unconditional) = if word.contains("::") {
                (outer_places(word, namespace), call.site.is_some())
            } else {
                match linked.get(word) {
                    None | Some(LinkedLocal::Caller) => continue,
                    Some(LinkedLocal::Unnameable) => return None,
                    Some(LinkedLocal::Outer(place, link)) => (
                        vec![place.clone()],
                        call.site
                            .is_some_and(|(block, index)| link.0 == block && link.1 < index),
                    ),
                }
            };
            found.push(OuterArgument {
                callee: callee.clone(),
                param,
                places,
                unconditional,
            });
        }
        Some(())
    }

    /// Classify only original selected source operations and original
    /// procedure allocations. Catalogue assistance cannot close this graph.
    fn callee_at(&self, tokens: &CommandTokens, callees: &mut BTreeSet<String>) -> Option<()> {
        let source = self.source.as_ref()?;
        if source.procedure_call(tokens).is_some() {
            callees.insert(
                self.resolve_call_at(tokens, FoldTrust::ObservedBindings)?
                    .procedure
                    .qualified_name
                    .clone(),
            );
            return Some(());
        }
        source.operation(tokens).map(|_| ())
    }

    /// Summarise one strongly connected set of procedures, whose callees
    /// outside it are summarised already.
    fn summarise_component(
        &self,
        component: &[&str],
        calls: &HashMap<&str, BTreeSet<String>>,
        inputs: &ModuleInputs<'_>,
        rounds: RecursionLimit,
    ) {
        let recursive = component.len() > 1
            || component
                .first()
                .is_some_and(|qname| calls[qname].contains(*qname));
        let mut roles: Vec<Option<Vec<ParamRole>>> = vec![None; component.len()];
        let mut settled = !recursive;
        if recursive {
            for qname in component {
                self.provisional
                    .borrow_mut()
                    .insert((*qname).to_owned(), None);
            }
            for round in 1.. {
                if rounds.exceeded(round) {
                    break;
                }
                let next: Vec<Option<Vec<ParamRole>>> = component
                    .iter()
                    .map(|qname| self.roles(qname, inputs))
                    .collect();
                let changed = next != roles;
                roles = next;
                for (qname, roles) in component.iter().zip(&roles) {
                    self.provisional
                        .borrow_mut()
                        .insert((*qname).to_owned(), roles.clone());
                }
                if !changed {
                    settled = true;
                    break;
                }
            }
            for qname in component {
                self.provisional.borrow_mut().remove(*qname);
            }
        } else if let Some(qname) = component.first() {
            roles[0] = self.roles(qname, inputs);
        }
        let Some(outer) = self.component_outer_writes(component, calls, inputs) else {
            return;
        };
        for (index, qname) in component.iter().enumerate() {
            let Some(declared) = inputs.ir.procedures.get(*qname) else {
                continue;
            };
            let (params, completion) = if !settled {
                (
                    linked_roles(declared, inputs, &ANY_OUTCOME),
                    CompletionCodeDomain::Any,
                )
            } else if let Some(roles) = roles[index].take() {
                (roles, self.completion_of(calls, qname))
            } else {
                (
                    linked_roles(declared, inputs, &[ExistenceOutcome::Preserve]),
                    CompletionCodeDomain::Exact(ERROR_ONLY),
                )
            };
            let Some(effects) = self.effects_of(qname, calls, inputs) else {
                continue;
            };
            let summary = TransferSummary {
                params,
                globals: outer.clone(),
                result: ReturnKind::Other,
                completion,
                effects,
                evidence: self.evidence_of(qname, calls),
                links: links_of(declared, inputs),
            };
            self.summaries
                .borrow_mut()
                .insert((*qname).to_owned(), summary);
        }
    }

    /// The completion codes a call to `qname` may end with: an error or a
    /// normal completion, once every callee completes so.
    fn completion_of(
        &self,
        calls: &HashMap<&str, BTreeSet<String>>,
        qname: &str,
    ) -> CompletionCodeDomain {
        let summaries = self.summaries.borrow();
        let exact = calls[qname].iter().all(|callee| {
            callee == qname
                || summaries.get(callee).is_none_or(|summary| {
                    matches!(summary.completion, CompletionCodeDomain::Exact(_))
                })
        });
        if exact {
            CompletionCodeDomain::Exact(NORMAL_OR_ERROR)
        } else {
            CompletionCodeDomain::Any
        }
    }

    /// The world effects of the commands `qname` runs as statements, with
    /// its callees' summarised effects.
    fn effects_of(
        &self,
        qname: &str,
        calls: &HashMap<&str, BTreeSet<String>>,
        _inputs: &ModuleInputs<'_>,
    ) -> Option<EffectFootprint> {
        let source = self.source.as_ref()?;
        let mut effects = EffectFootprint::default();
        for tokens in source.body_commands(qname)? {
            if source.procedure_call(&tokens).is_none() {
                effects.extend(
                    source
                        .operation(&tokens)?
                        .facts
                        .world_state_effects()
                        .clone(),
                );
            }
        }
        let summaries = self.summaries.borrow();
        for callee in &calls[qname] {
            if let Some(summary) = summaries.get(callee) {
                effects.extend(summary.effects.clone());
            }
        }
        Some(effects)
    }

    /// The bindings `qname`'s summary rests on: each procedure it calls,
    /// and what each of their summaries rests on.
    fn evidence_of(
        &self,
        qname: &str,
        calls: &HashMap<&str, BTreeSet<String>>,
    ) -> DependencyEvidence {
        let summaries = self.summaries.borrow();
        let mut bindings: Vec<BindingIdentity> = Vec::new();
        for callee in &calls[qname] {
            bindings.push(procedure_binding(callee));
            if let Some(summary) = summaries.get(callee) {
                bindings.extend(summary.evidence.bindings.iter().cloned());
            }
        }
        bindings.sort_unstable_by(|a, b| a.name.cmp(&b.name));
        bindings.dedup();
        DependencyEvidence {
            bindings,
            ..DependencyEvidence::default()
        }
    }

    /// The global and namespace places a set of procedures may write: those
    /// its bodies write, those its callees' summaries state, and those a
    /// callee's `Name` parameter names through a qualified argument or a
    /// local linked to a namespace's variable ([`Self::outer_arguments`]).
    /// A place one call names, which every normal completion makes and
    /// nothing else in the set writes, takes the callee's outcomes on it;
    /// every other is may-bound afterwards, and may-unbound too where a body
    /// the set runs destroys a variable or a callee may unbind such a place.
    fn component_outer_writes(
        &self,
        component: &[&str],
        calls: &HashMap<&str, BTreeSet<String>>,
        inputs: &ModuleInputs<'_>,
    ) -> Option<Vec<(PlaceRef, ExistenceOutcome)>> {
        let mut names: BTreeSet<String> = BTreeSet::new();
        let mut precise: Vec<(String, Vec<ExistenceOutcome>)> = Vec::new();
        let mut destroys = false;
        let summaries = self.summaries.borrow();
        for qname in component {
            if let Some(writes) = inputs.outer_writes.get(*qname) {
                let (namespace, _) = tcl_syntax::naming::key_holder_and_tail(qname);
                for name in &writes.names {
                    names.extend(outer_places(name, namespace));
                }
            }
            destroys |= destroys_a_variable(self.source.as_ref()?, qname)?;
            for callee in &calls[qname] {
                let Some(summary) = summaries.get(callee) else {
                    continue;
                };
                destroys |= summary
                    .globals
                    .iter()
                    .any(|(_, outcome)| *outcome == ExistenceOutcome::Unbind);
                names.extend(summary.globals.iter().map(|(place, _)| place.name.clone()));
            }
            for argument in self.outer_arguments(qname, inputs)? {
                let outcomes = summaries.get(&argument.callee).and_then(|summary| {
                    match summary.params.get(argument.param) {
                        Some(ParamRole::Name { outcomes, .. }) => Some(outcomes.clone()),
                        _ => None,
                    }
                });
                match (argument.unconditional, argument.places.as_slice(), outcomes) {
                    (true, [place], Some(outcomes)) if component.len() == 1 => {
                        precise.push((place.clone(), outcomes));
                    }
                    (_, _, outcomes) => {
                        // A callee that may unbind the place destroys a
                        // variable the set may write.
                        destroys |= outcomes
                            .is_none_or(|outcomes| outcomes.contains(&ExistenceOutcome::Unbind));
                        names.extend(argument.places);
                    }
                }
            }
        }
        let shared: Vec<bool> = precise
            .iter()
            .map(|(place, _)| {
                names.contains(place)
                    || precise.iter().filter(|(other, _)| other == place).count() > 1
            })
            .collect();
        destroys |= precise
            .iter()
            .zip(&shared)
            .any(|((_, outcomes), &shared)| shared && outcomes.contains(&ExistenceOutcome::Unbind));
        let mut places: BTreeMap<String, Vec<ExistenceOutcome>> = BTreeMap::new();
        for ((place, outcomes), shared) in precise.into_iter().zip(shared) {
            if destroys || shared {
                names.insert(place);
            } else {
                places.insert(place, outcomes);
            }
        }
        for name in names {
            let mut outcomes = Vec::new();
            if destroys {
                outcomes.push(ExistenceOutcome::Unbind);
            }
            outcomes.push(ExistenceOutcome::MayBind(BindingKind::Either));
            places.insert(name, outcomes);
        }
        Some(
            places
                .into_iter()
                .flat_map(|(name, outcomes)| {
                    let place = PlaceRef::scalar(name);
                    outcomes
                        .into_iter()
                        .map(move |outcome| (place.clone(), outcome))
                })
                .collect(),
        )
    }

    /// `qname`'s parameter roles under the summaries in hand: `None` when no
    /// normal exit of its body is reached.
    fn roles(&self, qname: &str, inputs: &ModuleInputs<'_>) -> Option<Vec<ParamRole>> {
        let declared = inputs.ir.procedures.get(qname)?;
        let cfg = inputs.cfg.procedures.get(qname)?;
        let links = links_of(declared, inputs);
        if links.is_empty() {
            return Some(value_roles(declared, inputs));
        }
        let ssa = crate::ssa::build_ssa_with_context_for_entry_and_metadata(
            cfg,
            inputs.registry,
            inputs.config,
            crate::compilation_unit::function_source_entry(cfg, None),
            Some(&declared.params),
            crate::registry_invocation::InvocationMetadataContext::for_module(
                inputs.registry,
                inputs.ir,
            ),
        );
        let owned: HashSet<String> = links.iter().map(|(_, local)| local.clone()).collect();
        let runs = [Existence::Bound(BindingKind::Either), Existence::Unbound]
            .map(|entry| self.summary_run(qname, (cfg, &ssa), &owned, entry, inputs));
        // Where each linked local stands at the normal exits of each run:
        // `None` for a run that completes nowhere — no normal exit reached,
        // or every path to one through a callee that never completes.
        let facts = runs.each_ref().map(|run| {
            let exits = normal_exits(cfg, run);
            if exits.is_empty() {
                return None;
            }
            links
                .iter()
                .map(|(_, local)| exit_fact(&ssa, run, &exits, local))
                .collect::<Option<Vec<Existence>>>()
        });
        if facts.iter().all(Option::is_none) {
            return None;
        }
        let reached: HashSet<crate::cfg::BlockId> = runs
            .iter()
            .flat_map(|run| run.executable_blocks.iter().copied())
            .collect();
        let mut roles = value_roles(declared, inputs);
        for (slot, (index, local)) in links.iter().enumerate() {
            let param = &declared.params[*index];
            let outcomes = if linked_cleanly(cfg, &ssa, local, param, qname, self.source.as_ref()?)
            {
                let [bound, unbound] = facts
                    .each_ref()
                    .map(|facts| facts.as_ref().map(|facts| facts[slot]));
                outcomes_of(bound, unbound, defined_in(&ssa, &reached, local))
            } else {
                ANY_OUTCOME.to_vec()
            };
            roles[*index] = ParamRole::Name {
                level: FrameLevel::Relative(1),
                outcomes,
            };
        }
        Some(roles)
    }

    /// One run of `qname`'s body for its summary, with its linked locals
    /// its own and entering as `entry` says.
    fn summary_run(
        &self,
        qname: &str,
        (cfg, ssa): (&CfgFunction, &SsaFunction),
        owned: &HashSet<String>,
        entry: Existence,
        inputs: &ModuleInputs<'_>,
    ) -> SccpResult {
        let declared = &inputs.ir.procedures[qname];
        let places = CallerPlaces::each(owned, entry);
        let trace = inputs.trace;
        crate::sccp::sccp_in_module(&SolveInputs {
            cfg,
            ssa,
            param_constants: None,
            policy: crate::tcl_expr_eval::FoldPolicy::from_registry(inputs.registry),
            extra_escaping: &HashSet::new(),
            trace: TraceInputs {
                source_metadata_input: inputs.ir.source_metadata_input.as_ref(),
                registry: inputs.registry,
                traced_variables: trace.traced_variables,
                has_dynamic_variable_trace: trace.has_dynamic_variable_trace,
                deferred_writes: trace.deferred_writes,
                analysis_context: Some(inputs.analysis_context),
                existence: Some(ExistenceEntry {
                    params: &declared.params,
                    object_state: None,
                    initial_global: false,
                    connection_scoped: None,
                    dynamic_trace: trace.has_dynamic_variable_trace || trace.deferred_writes.any,
                    config: inputs.config,
                    caller_places: Some(&places),
                }),
            },
            folds: Some(BuiltinFoldInputs {
                source_metadata_input: inputs.ir.source_metadata_input.as_ref(),
                registry: inputs.registry,
                mutations: inputs.mutations,
                dialect: None,
                defining_class: None,
                registry_engine: false,
                trust: FoldTrust::ObservedBindings,
                proven_pure_parameters: false,
            }),
            module: ModuleRun {
                procedures: Some(self),
                owned: Some(owned),
                level: ModuleLevel::Outcomes,
                reads_exits: true,
            },
        })
    }
}

/// Each parameter of `declared` whose value names the place one frame up
/// its body links a local to, by index, with that local.
fn links_of(declared: &crate::ir::Procedure, inputs: &ModuleInputs<'_>) -> Vec<(usize, String)> {
    let mut links: Vec<(usize, String)> = inputs
        .frames
        .get(&declared.qualified_name)
        .map(|frame| {
            frame
                .param_targets
                .iter()
                .filter_map(|(local, param)| {
                    let index = declared.params.iter().position(|p| p == param)?;
                    Some((index, local.clone()))
                })
                .collect()
        })
        .unwrap_or_default();
    links.sort_unstable();
    links
}

/// One `Name` argument of a call a procedure makes that names a place
/// outside the procedure's frame: a qualified name, or a local the
/// procedure links to a namespace's variable.
#[derive(Debug, Clone)]
struct OuterArgument {
    /// The callee, by qualified name.
    callee: String,
    /// The `Name` parameter's index.
    param: usize,
    /// The places the argument may name: one, or for a relative name its
    /// namespace's and the global namespace's.
    places: Vec<String>,
    /// Whether every normal completion of the procedure makes the call with
    /// the argument naming that place: a statement of the entry block, after
    /// the one link the argument reads.
    unconditional: bool,
}

/// A call [`ModuleProcedures::outer_arguments`] reads: its head, each
/// argument's literal value (`None` for one that substitutes), and its
/// `(block, index)` where it is a statement of the entry block.
#[derive(Clone, Copy)]
struct OuterCall<'c> {
    tokens: &'c CommandTokens,
    site: Option<(crate::cfg::BlockId, usize)>,
}

/// Where a local a procedure links outside its frame lives.
#[derive(Debug, Clone, PartialEq, Eq)]
enum LinkedLocal {
    /// A namespace's variable, by qualified name, linked by the statement at
    /// `(block, index)`.
    Outer(String, (crate::cfg::BlockId, usize)),
    /// The place one frame up a `Name` parameter names, which the
    /// parameter's role states.
    Caller,
    /// A place the summary cannot name.
    Unnameable,
}

/// Each local `cfg` links outside its frame, with where it lives
/// ([`LinkedLocal`]), from the registry's alias declarations and scope-alias
/// plans; a local linked twice, to two places, is unnameable.
fn linked_locals(
    cfg: &CfgFunction,
    namespace: &str,
    qname: &str,
    source: &SourceSummaryContext<'_>,
) -> Option<HashMap<String, LinkedLocal>> {
    use tcl_registry::{CallerFrameSelection, VariableAliasTarget};
    let mut linked: HashMap<String, LinkedLocal> = HashMap::new();
    let mut record = |local: &str, place: LinkedLocal| {
        let local = local.to_owned();
        let same = |held: &LinkedLocal| match (held, &place) {
            (LinkedLocal::Outer(held, _), LinkedLocal::Outer(new, _)) => held == new,
            (held, new) => held == new,
        };
        match linked.get(&local) {
            Some(held) if same(held) => {}
            Some(_) => {
                linked.insert(local, LinkedLocal::Unnameable);
            }
            None => {
                linked.insert(local, place);
            }
        }
    };
    let mut blocks: Vec<_> = cfg.blocks.iter().collect();
    blocks.sort_unstable_by_key(|(id, _)| **id);
    for (&id, block) in blocks {
        for (index, statement) in block.statements.iter().enumerate() {
            if statement.synthetic_marker().is_some() {
                continue;
            }
            let at = (id, index);
            let tokens = source.statement_tokens(qname, statement)?;
            let (aliases, dialect) = alias_facts(&tokens, source)?;
            for alias in aliases {
                let Some(local) = alias.local.literal() else {
                    continue;
                };
                let place = match &alias.target {
                    VariableAliasTarget::Global { variable } => variable
                        .literal()
                        .map_or(LinkedLocal::Unnameable, |variable| {
                            LinkedLocal::Outer(in_namespace(variable, "::"), at)
                        }),
                    VariableAliasTarget::CurrentNamespace { variable } => variable
                        .literal()
                        .map_or(LinkedLocal::Unnameable, |variable| {
                            LinkedLocal::Outer(in_namespace(variable, namespace), at)
                        }),
                    VariableAliasTarget::Namespace {
                        namespace: named,
                        variable,
                    } => match (named.literal(), variable.literal()) {
                        (Some(named), Some(variable)) if named.starts_with("::") => {
                            LinkedLocal::Outer(in_namespace(variable, named), at)
                        }
                        _ => LinkedLocal::Unnameable,
                    },
                    VariableAliasTarget::CallerSelectedFrame { frame, variable } => {
                        let level = match frame {
                            CallerFrameSelection::DefaultCaller => Some(FrameLevel::Relative(1)),
                            CallerFrameSelection::Explicit(level) => {
                                level.literal().and_then(|level| {
                                    dialect.and_then(|dialect| {
                                        FrameLevel::parse_for_dialect(level, dialect)
                                    })
                                })
                            }
                        };
                        match (level, variable.literal()) {
                            (Some(FrameLevel::Absolute(0)), Some(variable)) => {
                                LinkedLocal::Outer(in_namespace(variable, "::"), at)
                            }
                            (Some(FrameLevel::Relative(1)), None) => LinkedLocal::Caller,
                            _ => LinkedLocal::Unnameable,
                        }
                    }
                };
                record(local, place);
            }
            if let Some(operation) = source.operation(&tokens) {
                let metadata = source.metadata(&tokens)?;
                let realm = tokens.source_binding.as_ref()?.invocation_realm()?;
                let frame = operation.with_metadata_schema(
                    source.registry(),
                    metadata,
                    realm,
                    |schema| Some(schema.semantics.value.alias_frame()),
                )?;
                if matches!(
                    frame,
                    Some(
                        tcl_registry::value_transfer::AliasFrame::Object
                            | tcl_registry::value_transfer::AliasFrame::Connection
                    )
                ) {
                    for &(index, role) in &operation.facts.arg_roles {
                        if role == tcl_registry::ArgRole::VarWrite {
                            let argument = operation
                                .facts
                                .argument_offset
                                .checked_add(usize::from(index))?;
                            let word = operation.argument_literal(argument)?;
                            record(&word, LinkedLocal::Unnameable);
                        }
                    }
                }
            }
        }
    }
    Some(linked)
}

/// `variable`'s qualified name as a variable of `namespace` names it: an
/// absolute name is itself, and a relative one is in `namespace`.
fn in_namespace(variable: &str, namespace: &str) -> String {
    if variable.starts_with("::") {
        variable.to_owned()
    } else if namespace.is_empty() || namespace == "::" {
        format!("::{variable}")
    } else {
        format!("{namespace}::{variable}")
    }
}

/// The constant `local` holds at every one of a run's normal `exits`: the
/// value of each version that reaches an exit along the executable edges,
/// where every one agrees and may be written into source.
fn exit_constant(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    result: &SccpResult,
    exits: &[crate::cfg::BlockId],
    local: &str,
) -> Option<ExactValue> {
    let symbol = ssa.var_symbol(local)?;
    let mut found: Option<ExactValue> = None;
    for &exit in exits {
        for version in reaching_versions(cfg, ssa, result, exit, symbol) {
            let key = (symbol, version);
            let Some(crate::analyses::LatticeValue::Const(value)) = result.value_at(exit, key)
            else {
                return None;
            };
            if !result.materialises(key) {
                return None;
            }
            let value = crate::value_transfer::const_to_exact(value);
            match &found {
                Some(held) if held.bytes != value.bytes => return None,
                Some(_) => {}
                None => found = Some(value),
            }
        }
    }
    found
}

/// The versions of `symbol` that reach the end of `block`: the block's own
/// last definition or φ, else each executable predecessor's, back to the
/// version the function enters with.
fn reaching_versions(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    result: &SccpResult,
    block: crate::cfg::BlockId,
    symbol: crate::ssa::Symbol,
) -> BTreeSet<crate::ssa::Version> {
    let preds = cfg.predecessors();
    let mut versions = BTreeSet::new();
    let mut seen: HashSet<crate::cfg::BlockId> = HashSet::new();
    let mut work = vec![block];
    while let Some(at) = work.pop() {
        if !seen.insert(at) {
            continue;
        }
        let own = ssa.blocks.get(&at).and_then(|ssa_block| {
            ssa_block
                .statements
                .iter()
                .rev()
                .find_map(|statement| statement.defs.get(&symbol).copied())
                .or_else(|| {
                    ssa_block
                        .phis
                        .iter()
                        .find(|phi| phi.name == symbol)
                        .map(|phi| phi.version)
                })
        });
        if let Some(version) = own {
            versions.insert(version);
            continue;
        }
        let incoming: Vec<crate::cfg::BlockId> = preds
            .get(&at)
            .into_iter()
            .flatten()
            .copied()
            .filter(|pred| result.executable_edges.contains(&(*pred, at)))
            .collect();
        if incoming.is_empty() {
            versions.insert(0);
        }
        work.extend(incoming);
    }
    versions
}

/// Whether a procedure's caller-frame effects are all `Name` links its
/// summary can follow: each a level-1 `upvar` of one parameter's value onto
/// one local of its own, with no other frame reached.
fn frame_is_closed(frame: &UpvarInfo, params: &[String]) -> bool {
    let mut linked: HashSet<&str> = HashSet::new();
    let one_local_each = frame
        .param_targets
        .iter()
        .all(|(local, param)| !params.contains(local) && linked.insert(param.as_str()));
    one_local_each
        && !frame.has_unresolvable_caller_target
        && !frame.caller_frame_opaque_writes
        && !frame.caller_frame_opaque_reads
        && frame.frame_reach == FrameReach::NoFurtherThanTheCaller
        && frame.literal_targets.is_empty()
        && frame.args_tail_upvar.is_empty()
        && frame.uplevel_literal_writes.is_empty()
        && frame.uplevel_param_writes.is_empty()
        && frame.uplevel_forwarded_calls.is_empty()
        && frame.unnameable_local_aliases.is_empty()
}

/// The places an outer name a procedure in `namespace` writes may be: an
/// absolute name is one place; a relative one is in the global namespace
/// for a procedure there, and elsewhere either the procedure's namespace's
/// (a `variable`) or the global one's (a `global`).
fn outer_places(name: &str, namespace: &str) -> Vec<String> {
    if name.starts_with("::") {
        return vec![name.to_owned()];
    }
    let global = format!("::{name}");
    if namespace.is_empty() || namespace == "::" {
        vec![global]
    } else {
        vec![format!("{namespace}::{name}"), global]
    }
}

/// Whether some statement of `cfg` may destroy a variable.
fn destroys_a_variable(source: &SourceSummaryContext<'_>, qname: &str) -> Option<bool> {
    let mut destroys = false;
    for tokens in source.body_commands(qname)? {
        if source.procedure_call(&tokens).is_none() {
            destroys |= source
                .operation(&tokens)?
                .facts
                .traits
                .contains(tcl_registry::Traits::DESTROYS_VARIABLE);
        }
    }
    Some(destroys)
}

/// Parameter uses from the existing SSA owner under the complete retained
/// metadata and lexer configuration. This is reached only after the source
/// graph/frame/dynamic-name guards close the summary; a missing graph cannot
/// establish that a parameter is unused.
fn value_roles(declared: &crate::ir::Procedure, inputs: &ModuleInputs<'_>) -> Vec<ParamRole> {
    let Some(cfg) = inputs.cfg.procedures.get(&declared.qualified_name) else {
        return vec![ParamRole::Value; declared.params.len()];
    };
    let ssa = crate::ssa::build_ssa_with_context_for_entry_and_metadata(
        cfg,
        inputs.registry,
        inputs.config,
        crate::compilation_unit::function_source_entry(cfg, None),
        Some(&declared.params),
        crate::registry_invocation::InvocationMetadataContext::for_module(
            inputs.registry,
            inputs.ir,
        ),
    );
    declared
        .params
        .iter()
        .map(|parameter| {
            if ssa.var_symbol(parameter).is_some() {
                ParamRole::Value
            } else {
                ParamRole::Unused
            }
        })
        .collect()
}

/// [`value_roles`], with every parameter the body links a local to a place
/// the body applies `outcomes` to: the roles of a procedure that never
/// completes normally, or of a cycle that did not settle.
fn linked_roles(
    declared: &crate::ir::Procedure,
    inputs: &ModuleInputs<'_>,
    outcomes: &[ExistenceOutcome],
) -> Vec<ParamRole> {
    let linked: HashSet<&str> = inputs
        .frames
        .get(&declared.qualified_name)
        .map(|frame| frame.param_targets.values().map(String::as_str).collect())
        .unwrap_or_default();
    value_roles(declared, inputs)
        .into_iter()
        .zip(&declared.params)
        .map(|(role, param)| {
            if linked.contains(param.as_str()) {
                ParamRole::Name {
                    level: FrameLevel::Relative(1),
                    outcomes: outcomes.to_vec(),
                }
            } else {
                role
            }
        })
        .collect()
}

/// The blocks a run reaches whose end completes the procedure normally: a
/// block a statement of which certainly raises never reaches its end.
fn normal_exits(cfg: &CfgFunction, result: &SccpResult) -> Vec<crate::cfg::BlockId> {
    let mut exits: Vec<crate::cfg::BlockId> = result
        .executable_blocks
        .iter()
        .copied()
        .filter(|id| {
            !result.completion.raises_in(*id)
                && cfg.blocks.get(id).is_some_and(|block| {
                    matches!(block.terminator, None | Some(Terminator::Return { .. }))
                })
        })
        .collect();
    exits.sort_unstable();
    exits
}

/// The fact `local` holds where a run's normal exits leave it, joined;
/// `None` where no exit was reached or the fact waits on a callee that has
/// not completed.
fn exit_fact(
    ssa: &SsaFunction,
    result: &SccpResult,
    exits: &[crate::cfg::BlockId],
    local: &str,
) -> Option<Existence> {
    let symbol = ssa.var_symbol(local)?;
    let fact = exits
        .iter()
        .filter_map(|block| {
            result
                .existence_exits
                .get(block)?
                .get(symbol.0 as usize)
                .copied()
        })
        .fold(Existence::Pending, Existence::join);
    (fact != Existence::Pending).then_some(fact)
}

/// Whether a block a run reached defines `local`, by a statement or a φ.
fn defined_in(ssa: &SsaFunction, reached: &HashSet<crate::cfg::BlockId>, local: &str) -> bool {
    let Some(symbol) = ssa.var_symbol(local) else {
        return false;
    };
    reached
        .iter()
        .filter_map(|block| ssa.blocks.get(block))
        .any(|block| {
            block.phis.iter().any(|phi| phi.name == symbol)
                || block
                    .statements
                    .iter()
                    .any(|statement| statement.defs.contains_key(&symbol))
        })
}

/// Whether `local` is linked once, by the entry block's first statement
/// that touches it, to the place the parameter's own value names, and is
/// only ever reached as a whole variable.
fn linked_cleanly(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    local: &str,
    param: &str,
    qname: &str,
    source: &SourceSummaryContext<'_>,
) -> bool {
    if ssa.var_names().iter().any(|name| {
        tcl_syntax::naming::split_element_ref_bytes(name.as_bytes())
            .is_some_and(|(root, _)| root == local.as_bytes())
    }) {
        return false;
    }
    if cfg
        .blocks
        .values()
        .flat_map(|block| &block.statements)
        .any(|statement| links_local(statement, local, qname, source).is_none())
    {
        return false;
    }
    let sites: Vec<(crate::cfg::BlockId, usize)> = cfg
        .blocks
        .iter()
        .flat_map(|(id, block)| {
            block
                .statements
                .iter()
                .enumerate()
                .filter(|(_, statement)| {
                    links_local(statement, local, qname, source).unwrap_or(false)
                })
                .map(move |(index, _)| (*id, index))
        })
        .collect();
    let [(block, index)] = sites[..] else {
        return false;
    };
    let (Some(symbol), Some(param_symbol), Some(entry)) = (
        ssa.var_symbol(local),
        ssa.var_symbol(param),
        ssa.blocks.get(&cfg.entry),
    ) else {
        return false;
    };
    block == cfg.entry
        && entry.statements[..index].iter().all(|statement| {
            !statement.uses.contains_key(&symbol) && !statement.defs.contains_key(&symbol)
        })
        && entry.statements[index].uses.get(&param_symbol) == Some(&0)
}

/// Alias declarations from the same selected original argv, including captures.
fn alias_facts(
    tokens: &CommandTokens,
    source: &SourceSummaryContext<'_>,
) -> Option<(
    Vec<tcl_registry::VariableCellAliasTransition>,
    Option<tcl_registry::InvocationDialect>,
)> {
    if source.procedure_call(tokens).is_some() {
        return Some((Vec::new(), None));
    }
    let operation = source.operation(tokens)?;
    let transitions = operation.facts.state_transitions.declared()?;
    Some((
        transitions
            .facts()
            .iter()
            .filter_map(|fact| match &fact.transition {
                tcl_registry::StateTransition::VariableCellAlias(alias) => Some(alias.clone()),
                _ => None,
            })
            .collect(),
        operation.dialect,
    ))
}

/// A computed outside-cell target prevents a closed conditional summary.
fn aliases_unnamed_place(
    tokens: &CommandTokens,
    source: &SourceSummaryContext<'_>,
) -> Option<bool> {
    use tcl_registry::{CallerFrameSelection, VariableAliasTarget};
    let (aliases, dialect) = alias_facts(tokens, source)?;
    Some(aliases.iter().any(|alias| match &alias.target {
        VariableAliasTarget::Global { variable }
        | VariableAliasTarget::CurrentNamespace { variable } => variable.literal().is_none(),
        VariableAliasTarget::Namespace {
            namespace,
            variable,
        } => namespace.literal().is_none() || variable.literal().is_none(),
        VariableAliasTarget::CallerSelectedFrame { frame, variable } => {
            let caller = match frame {
                CallerFrameSelection::DefaultCaller => true,
                CallerFrameSelection::Explicit(level) => level
                    .literal()
                    .and_then(|level| {
                        dialect.and_then(|dialect| FrameLevel::parse_for_dialect(level, dialect))
                    })
                    .is_some_and(|level| level == FrameLevel::Relative(1)),
            };
            !caller && variable.literal().is_none()
        }
    }))
}

fn links_local(
    statement: &Statement,
    local: &str,
    qname: &str,
    source: &SourceSummaryContext<'_>,
) -> Option<bool> {
    if statement.synthetic_marker().is_some() {
        return Some(false);
    }
    let tokens = source.statement_tokens(qname, statement)?;
    let (aliases, _) = alias_facts(&tokens, source)?;
    Some(aliases.iter().any(|alias| {
        alias.local.literal().is_some_and(|name| {
            tcl_syntax::naming::split_element_ref_bytes(name.as_bytes()).is_none() && name == local
        })
    }))
}

/// The outcomes a body applies to a linked place, from where the two runs
/// leave it — bound on entry, then unbound — and whether any exit holds a
/// version the body wrote.
fn outcomes_of(
    bound: Option<Existence>,
    unbound: Option<Existence>,
    written: bool,
) -> Vec<ExistenceOutcome> {
    if !written {
        return vec![ExistenceOutcome::Preserve];
    }
    let facts: Vec<Existence> = [bound, unbound].into_iter().flatten().collect();
    if facts.is_empty() {
        return vec![ExistenceOutcome::Preserve];
    }
    if facts.iter().all(|fact| matches!(fact, Existence::Bound(_))) {
        let kind = facts
            .iter()
            .filter_map(|fact| match fact {
                Existence::Bound(kind) => Some(*kind),
                _ => None,
            })
            .reduce(BindingKind::join)
            .unwrap_or(BindingKind::Either);
        return vec![ExistenceOutcome::Bind(kind)];
    }
    if facts.iter().all(|fact| *fact == Existence::Unbound) {
        return vec![ExistenceOutcome::Unbind];
    }
    if bound.is_none_or(|fact| matches!(fact, Existence::Bound(_))) {
        return vec![ExistenceOutcome::MayBind(BindingKind::Either)];
    }
    ANY_OUTCOME.to_vec()
}

/// The step a sequence of outcomes composes to.
fn step_of(outcomes: &[ExistenceOutcome]) -> ExistenceStep {
    outcomes
        .iter()
        .fold(ExistenceStep::PRESERVE, |step, outcome| {
            step.then(ExistenceStep::of(*outcome))
        })
}

/// The binding a call to `qname` rests on.
fn procedure_binding(qname: &str) -> BindingIdentity {
    BindingIdentity {
        resolution_namespace: String::new(),
        name: qname.to_owned(),
        identity: format!("proc {qname}"),
    }
}

/// The revision of a module's summaries: its procedures' names, parameter
/// lists, bodies and redefinitions, and its command bindings.
fn revision_of(ir: &crate::ir::Module, mutations: &ModuleCommandMutations) -> u64 {
    let mut hasher = rustc_hash::FxHasher::default();
    mutations.snapshot().hash(&mut hasher);
    let mut names: Vec<&String> = ir.procedures.keys().collect();
    names.sort_unstable();
    for name in names {
        let declared = &ir.procedures[name];
        name.hash(&mut hasher);
        declared.params_raw.hash(&mut hasher);
        declared.body_source.hash(&mut hasher);
    }
    let mut redefined: Vec<&String> = ir.redefined_procedures.iter().collect();
    redefined.sort_unstable();
    redefined.hash(&mut hasher);
    hasher.finish()
}

/// The strongly connected sets of `calls`' procedures, each after every set
/// it calls (Tarjan's algorithm, iterative).
fn components<'n>(
    names: &[&'n str],
    calls: &HashMap<&'n str, BTreeSet<String>>,
) -> Vec<Vec<&'n str>> {
    struct Node {
        index: usize,
        low: usize,
        on_stack: bool,
    }
    let mut nodes: HashMap<&str, Node> = HashMap::new();
    let mut stack: Vec<&str> = Vec::new();
    let mut out: Vec<Vec<&'n str>> = Vec::new();
    let mut next = 0usize;
    for &root in names {
        if !calls.contains_key(root) || nodes.contains_key(root) {
            continue;
        }
        // Each frame: the node and the successors still to visit.
        let mut work: Vec<(&'n str, Vec<&'n str>)> = Vec::new();
        let successors = |qname: &str| -> Vec<&'n str> {
            calls[qname]
                .iter()
                .rev()
                .filter_map(|callee| calls.get_key_value(callee.as_str()).map(|(key, _)| *key))
                .collect()
        };
        nodes.insert(
            root,
            Node {
                index: next,
                low: next,
                on_stack: true,
            },
        );
        next += 1;
        stack.push(root);
        work.push((root, successors(root)));
        while let Some((node, pending)) = work.last_mut() {
            let node = *node;
            if let Some(callee) = pending.pop() {
                match nodes.get(callee) {
                    None => {
                        nodes.insert(
                            callee,
                            Node {
                                index: next,
                                low: next,
                                on_stack: true,
                            },
                        );
                        next += 1;
                        stack.push(callee);
                        work.push((callee, successors(callee)));
                    }
                    Some(seen) if seen.on_stack => {
                        let index = seen.index;
                        let entry = nodes.get_mut(node).expect("visited");
                        entry.low = entry.low.min(index);
                    }
                    Some(_) => {}
                }
                continue;
            }
            work.pop();
            let (index, low) = {
                let entry = &nodes[node];
                (entry.index, entry.low)
            };
            if let Some((parent, _)) = work.last() {
                let parent = nodes.get_mut(*parent).expect("visited");
                parent.low = parent.low.min(low);
            }
            if low == index {
                let mut component = Vec::new();
                while let Some(member) = stack.pop() {
                    nodes.get_mut(member).expect("visited").on_stack = false;
                    component.push(member);
                    if member == node {
                        break;
                    }
                }
                component.sort_unstable();
                out.push(component);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compilation_unit::CompilationUnit;

    fn name(outcomes: &[ExistenceOutcome]) -> ParamRole {
        ParamRole::Name {
            level: FrameLevel::Relative(1),
            outcomes: outcomes.to_vec(),
        }
    }

    const BIND: ExistenceOutcome = ExistenceOutcome::Bind(BindingKind::Scalar);

    /// The transfer summaries the build of `source` computes under the
    /// `environment` profile.
    fn summaries(source: &str, environment: &str) -> HashMap<String, TransferSummary> {
        crate::interprocedural::logical_completion_unit(source, environment)
            .1
            .transfers
            .0
    }

    /// The summaries of `source` with a cycle given at most `rounds` rounds.
    fn summaries_within(source: &str, rounds: RecursionLimit) -> HashMap<String, TransferSummary> {
        let (context_owner, unit) =
            crate::interprocedural::logical_completion_unit(source, "tcl8.6");
        let registry = context_owner.commands();
        let prepared = crate::cfg_builder::prepare_cfg_context_bundle(&unit.ir_module, registry);
        let context = crate::value_transfer::AnalysisContextKey::for_module(
            &unit.command_mutations,
            registry,
        );
        ModuleProcedures::with_rounds(
            ModuleInputs {
                ir: &unit.ir_module,
                cfg: &unit.cfg_module,
                frames: &prepared.context.0,
                outer_writes: &prepared.context.2,
                registry,
                mutations: &unit.command_mutations,
                projection: &unit.caller_scope.proc_binding_trust,
                trace: crate::compilation_unit::ModuleTraceFacts::of(&unit.ir_module),
                config: unit.ir_module.lexer_config,
                analysis_context: &context,
            },
            rounds,
        )
        .into_summaries()
        .0
    }

    const CALLEES: &str = "proc bump {name {by 1}} {upvar 1 $name v; incr v $by}\n\
        proc reset {name} {upvar 1 $name v; unset v}\n\
        proc twice {name} {upvar 1 $name w; bump w; bump w}\n\
        proc renew {name} {upvar 1 $name x; reset x; bump x}\n\
        proc clear {name} {upvar 1 $name y; bump y; reset y}\n\
        proc maybe {name c} {upvar 1 $name v; if {$c} {set v 1}}\n\
        proc ensure {name} {upvar 1 $name v; if {![info exists v]} {set v 0}}\n\
        proc peek {name} {upvar 1 $name v; return $v}\n";

    /// A summary composes its callees' outcomes in the order the body
    /// applies them: `twice`'s place is the two `bump` updates, a bind;
    /// `reset` then `bump` binds and `bump` then `reset` unbinds. Each
    /// procedure's two runs — the place bound on entry, then unbound — tell
    /// a may-write (`maybe`) from a write whatever the prior (`ensure`), and
    /// a place the body only reads is preserved. The composition is the
    /// same under every release: `incr` of an unset variable raises in 8.4
    /// and creates it from 8.5, and either way a normal completion leaves
    /// the place bound.
    #[test]
    fn summaries_compose_through_two_callees() {
        for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let summaries = summaries(CALLEES, environment);
            let params = |qname: &str| -> Vec<ParamRole> {
                summaries
                    .get(qname)
                    .unwrap_or_else(|| panic!("{environment}: {qname} has a summary"))
                    .params
                    .clone()
            };
            assert_eq!(
                params("::bump"),
                [name(&[BIND]), ParamRole::Value],
                "{environment}"
            );
            assert_eq!(
                params("::reset"),
                [name(&[ExistenceOutcome::Unbind])],
                "{environment}"
            );
            assert_eq!(params("::twice"), [name(&[BIND])], "{environment}");
            assert_eq!(params("::renew"), [name(&[BIND])], "{environment}");
            assert_eq!(
                params("::clear"),
                [name(&[ExistenceOutcome::Unbind])],
                "{environment}"
            );
            assert_eq!(
                params("::maybe"),
                [
                    name(&[ExistenceOutcome::MayBind(BindingKind::Either)]),
                    ParamRole::Value
                ],
                "{environment}"
            );
            assert_eq!(
                params("::ensure"),
                [name(&[ExistenceOutcome::Bind(BindingKind::Either)])],
                "{environment}"
            );
            assert_eq!(
                params("::peek"),
                [name(&[ExistenceOutcome::Preserve])],
                "{environment}"
            );
            let twice = &summaries["::twice"];
            assert_eq!(
                twice
                    .evidence
                    .bindings
                    .iter()
                    .map(|binding| binding.name.as_str())
                    .collect::<Vec<_>>(),
                ["::bump"],
                "{environment}: `twice` rests on `bump`"
            );
            assert_eq!(
                twice.completion,
                CompletionCodeDomain::Exact(NORMAL_OR_ERROR),
                "{environment}"
            );
            assert!(twice.globals.is_empty(), "{environment}");
        }
    }

    /// A procedure that reaches code the module cannot see, or a frame its
    /// summary cannot name, has no summary, and neither has a caller of it.
    #[test]
    fn a_barrier_has_no_summary() {
        let source = "proc opaque {name} {upvar 1 $name v; mystery; incr v}\n\
            proc far {name} {upvar 2 $name v; incr v}\n\
            proc outer {name} {upvar 1 $name v; opaque v}\n\
            proc global_write {} {set ::counter 5}\n";
        let summaries = summaries(source, "tcl8.6");
        for barrier in ["::opaque", "::far", "::outer"] {
            assert!(!summaries.contains_key(barrier), "{barrier}: {summaries:?}");
        }
        let global_write = &summaries["::global_write"];
        assert_eq!(
            global_write.globals,
            [(
                PlaceRef::scalar("::counter"),
                ExistenceOutcome::MayBind(BindingKind::Either)
            )]
        );
    }

    /// A call that omits a `Name` parameter names the place the parameter's
    /// default spells, as Tcl binds the default; a word count the callee's
    /// parameters reject is a call that never completes; and a default that
    /// names an element is a barrier, as an element argument is.
    #[test]
    fn a_call_names_its_omitted_parameters_default() {
        let source = "proc bumpd {{name n} {by 1}} {upvar 1 $name v; incr v $by}\n\
            proc cell {{name a(1)}} {upvar 1 $name v; incr v}\n";
        let (context_owner, unit) =
            crate::interprocedural::logical_completion_unit(source, "tcl8.6");
        let registry = context_owner.commands();
        let module = ModuleProcedures::of_unit(&unit, registry);
        let bind = step_of(&[BIND]);
        assert_eq!(
            module.call_transfer("::bumpd", &[]),
            Some(CallTransfer::Places(vec![("n".to_owned(), bind)]))
        );
        assert_eq!(
            module.call_transfer("::bumpd", &[Some("m"), None]),
            Some(CallTransfer::Places(vec![("m".to_owned(), bind)]))
        );
        assert_eq!(
            module.call_transfer("::bumpd", &[Some("m"), Some("1"), Some("x")]),
            Some(CallTransfer::Never)
        );
        assert_eq!(module.call_transfer("::cell", &[]), None);
    }

    /// A `Name` argument naming a place outside the caller's frame — a
    /// qualified name, or a local the caller links to a namespace's
    /// variable by `upvar #0`, `global` or `variable` — is a write the
    /// caller's summary states among its globals: with the callee's outcome
    /// where every completion of the caller makes the call, may-bound where
    /// only some do, and may-unbound too where the callee may unbind it. A
    /// local linked to one of two places names a place the summary cannot,
    /// so that caller has no summary. tclsh 8.4 to 9.1 print 2 for `puts
    /// [c]`.
    #[test]
    fn a_name_argument_naming_an_outer_place_is_a_global_write() {
        let source = "proc bump {name {by 1}} {upvar 1 $name v; incr v $by}\n\
            set g 1\n\
            proc c {} {bump ::g; return $::g}\n\
            proc d {} {upvar #0 g x; bump x}\n\
            proc e {} {global g; bump g}\n\
            proc h {c} {if {$c} {bump ::g}}\n\
            proc f {c} {if {$c} {upvar #0 g x} else {upvar #0 k x}; bump x}\n\
            namespace eval ns {variable v 1; proc w {} {variable v; bump v}}\n\
            proc reset {name} {upvar 1 $name v; unset v}\n\
            proc r {} {reset ::q}\n\
            proc s {c} {if {$c} {reset ::q}}\n";
        let (context_owner, unit) =
            crate::interprocedural::logical_completion_unit(source, "tcl8.6");
        let registry = context_owner.commands();
        let module = ModuleProcedures::of_unit(&unit, registry);
        let g = PlaceRef::scalar("::g".to_owned());
        for caller in ["::c", "::d", "::e"] {
            assert_eq!(
                unit.transfers.0[caller].globals,
                vec![(g.clone(), BIND)],
                "{caller}"
            );
            assert!(!module.keeps_to_its_frame(caller), "{caller}");
        }
        assert_eq!(
            unit.transfers.0["::h"].globals,
            vec![(g, ExistenceOutcome::MayBind(BindingKind::Either))]
        );
        assert_eq!(
            unit.transfers.0["::ns::w"].globals,
            vec![(PlaceRef::scalar("::ns::v".to_owned()), BIND)]
        );
        let q = PlaceRef::scalar("::q".to_owned());
        assert_eq!(
            unit.transfers.0["::r"].globals,
            vec![(q.clone(), ExistenceOutcome::Unbind)]
        );
        assert_eq!(
            unit.transfers.0["::s"].globals,
            vec![
                (q.clone(), ExistenceOutcome::Unbind),
                (q, ExistenceOutcome::MayBind(BindingKind::Either))
            ]
        );
        assert!(!unit.transfers.0.contains_key("::f"));
        assert!(unit.transfers.0.contains_key("::bump"));
    }

    const CYCLE: &str =
        "proc up {name n} {upvar 1 $name v; if {$n > 0} {incr v; up v [expr {$n - 1}]}}\n";

    /// A cycle is solved from its procedures never completing, round by
    /// round: `up`, which passes its place to itself, settles in two rounds
    /// on a may-write of the place. Given one round, the cycle does not
    /// settle, and every place it names is may-bound afterwards whatever it
    /// held, with any completion and a computed result.
    #[test]
    fn a_cycle_that_does_not_converge_is_may_bind() {
        let settled = summaries_within(
            CYCLE,
            crate::interprocedural::MAX_INTERPROCEDURAL_WALK_DEPTH,
        );
        let summary = &settled["::up"];
        assert_eq!(
            summary.params,
            [
                name(&[ExistenceOutcome::MayBind(BindingKind::Either)]),
                ParamRole::Value
            ],
            "`up` settles"
        );
        assert_eq!(
            summary.completion,
            CompletionCodeDomain::Exact(NORMAL_OR_ERROR)
        );
        let unsettled = summaries_within(CYCLE, RecursionLimit(1));
        let summary = &unsettled["::up"];
        assert_eq!(
            summary.params,
            [name(&ANY_OUTCOME), ParamRole::Value],
            "`up` does not settle"
        );
        assert_eq!(summary.completion, CompletionCodeDomain::Any);
        assert_eq!(summary.result, ReturnKind::Other);
        assert_eq!(
            step_of(&ANY_OUTCOME).apply(Existence::Bound(BindingKind::Scalar)),
            Existence::MayBound
        );
    }

    /// The revision every summary rides on moves with a `rename` or a
    /// redefinition anywhere in the module, and with nothing else.
    #[test]
    fn a_rename_or_a_redefinition_moves_the_revision() {
        let revision = |source: &str| {
            let registry = CommandRegistry::build_default();
            let unit = CompilationUnit::build_for(source, &registry, false);
            revision_of(&unit.ir_module, &unit.command_mutations)
        };
        let base =
            "proc bump {name} {upvar 1 $name v; incr v}\nproc p {} {set n 1; bump n; return $n}\n";
        let same = revision(base);
        assert_eq!(same, revision(base));
        assert_ne!(same, revision(&format!("{base}rename bump bump2\n")));
        assert_ne!(same, revision(&format!("{base}proc bump {{name}} {{}}\n")));
        assert_ne!(same, revision(&base.replace("incr v", "incr v 2")));
    }

    #[test]
    fn transfer_formals_keep_selected_tcl_and_jim_binders_separate() {
        // naming.interprocedural.original-transfer-summary-source-context
        // docs/design/analysis/name-resolution-proofs/interprocedural-original-transfer-summary-source-context.md
        // Selected software formal binding, not native local slots or activation.
        let source = "proc optional {{a A} b} {return 1}\nproc middle {a args b} {return 1}\nproc linked {&v} {return 1}";
        for environment in ["tcl8.6", "jim"] {
            let (context, unit) =
                crate::interprocedural::logical_completion_unit(source, environment);
            let module = ModuleProcedures::of_unit(&unit, context.commands());
            let grammar = module.formals("::optional").unwrap().1;
            if grammar == tcl_dialect::ParameterGrammar::Jim {
                let values = module
                    .parameter_values("::optional", &[Some(ExactValue::from_literal("B"))])
                    .unwrap();
                assert_eq!(
                    values,
                    [
                        Some(ExactValue::from_literal("A")),
                        Some(ExactValue::from_literal("B"))
                    ]
                );
                let arguments: Vec<_> = ["A", "B", "C", "D"]
                    .into_iter()
                    .map(|value| Some(ExactValue::from_literal(value)))
                    .collect();
                assert_eq!(
                    module.parameter_values("::middle", &arguments).unwrap(),
                    [
                        Some(ExactValue::from_literal("A")),
                        Some(ExactValue::from_literal("B C")),
                        Some(ExactValue::from_literal("D"))
                    ]
                );
                assert!(
                    module
                        .parameter_values("::linked", &[Some(ExactValue::from_literal("caller"))])
                        .is_none()
                );
                assert!(!unit.transfers.0.contains_key("::linked"));
            } else {
                assert_eq!(grammar, tcl_dialect::ParameterGrammar::Tcl);
                assert!(
                    module
                        .parameter_values("::optional", &[Some(ExactValue::from_literal("B"))])
                        .is_none()
                );
                assert_eq!(
                    module
                        .parameter_values("::linked", &[Some(ExactValue::from_literal("value"))])
                        .unwrap(),
                    [Some(ExactValue::from_literal("value"))]
                );
            }
        }
    }

    #[test]
    fn transfer_unused_formals_use_actual_variable_roots_and_metadata() {
        // naming.interprocedural.original-transfer-summary-source-context
        // docs/design/analysis/name-resolution-proofs/interprocedural-original-transfer-summary-source-context.md
        // Conditional SSA use inventory, not native frame or replacement proof.
        let source = "proc p {{a b} {$tag} é unused} {return \"${a b}${$tag}${é}\"}";
        let result = summaries(source, "tcl8.6");
        assert_eq!(
            result["::p"].params,
            [
                ParamRole::Value,
                ParamRole::Value,
                ParamRole::Value,
                ParamRole::Unused
            ]
        );
        let array = summaries("proc p {a(k)} {return 1}", "tcl8.6");
        assert!(
            !array.contains_key("::p"),
            "formal syntax supplies no scalar activation closure"
        );
    }
}
