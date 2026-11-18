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
use std::collections::{HashMap, HashSet};
use std::hash::{Hash as _, Hasher as _};
use std::rc::Rc;

use crate::ir::CommandTokens;
use tcl_core_types::RecursionLimit;
use tcl_registry::CommandRegistry;
use tcl_registry::completion::{CompletionCode, CompletionCodeDomain};
use tcl_registry::frame_effect::FrameLevel;
#[cfg(test)]
use tcl_registry::value_transfer::Existence;
use tcl_registry::value_transfer::{
    BindingKind, DependencyEvidence, ExactValue, ExistenceOutcome, ParameterDefault, PlaceRef,
};
use tcl_registry::world_effect::EffectFootprint;
use tcl_syntax::formal_params::{FormalArgumentBinding, FormalParameter};

mod source_context;
use source_context::{NativeSummaryContext, SourceSummaryContext};
pub(crate) use source_context::{OriginalSummaryExpression, OriginalSummaryScript};

use crate::cfg_builder::global_write_info::GlobalWriteInfo;
use crate::cfg_builder::upvar_info::UpvarInfo;
use crate::command_binding::{ModuleCommandMutations, ProcBindingTrustProjection};
use crate::sccp::FoldTrust;
use crate::ssa::SsaFunction;
use crate::value_transfer::ExistenceStep;

use super::ReturnKind;
use crate::signature_scan::variable_symbol::SignatureSourceVariableOccurrence;

mod original_source;
mod outer_places;
mod parameter_roles;
mod reruns;
mod summarisation;

#[cfg(test)]
use outer_places::alias_facts;
use reruns::RerunKey;
pub(crate) use reruns::{Rerun, RerunStance};

type OuterSourcePlace = SignatureSourceVariableOccurrence;

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
    /// Conditional original source coordinates, independent of physical cells.
    global_sources: Vec<(OuterSourcePlace, ExistenceOutcome)>,
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
pub struct TransferSummaries(pub(crate) HashMap<crate::ir::ProcedureId, TransferSummary>);

impl TransferSummaries {
    /// `qname`'s summary as the Explorer prints it, its parameters named
    /// `params`: each parameter's role, the outer places it may write, and
    /// how a call may complete. `None` for a procedure with none, a call to
    /// which is a barrier.
    #[must_use]
    pub fn describe(&self, qname: &crate::ir::ProcedureId, params: &[String]) -> Option<String> {
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
    /// Each procedure's writes to global and namespace places.
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

impl<'a> ModuleInputs<'a> {
    /// `id`'s flow graph as its own declared frame runs it: the graph of its
    /// body lowered again in that frame where the walk also entered it from
    /// a caller, else the module's.
    pub(crate) fn summary_cfg(
        &self,
        id: &crate::ir::ProcedureId,
    ) -> Option<&'a crate::cfg::Function> {
        self.cfg
            .declared_frame_procedures
            .get(id)
            .or_else(|| self.cfg.procedures.get(id))
    }
}

/// The module's procedures as a lattice run reads them: their parameters
/// and defaults, whether a name still denotes one, and their transfer
/// summaries.
pub(crate) struct ModuleProcedures<'a> {
    /// Actual lowered source owner for fresh CFGs and procedure reruns.
    ir: &'a crate::ir::Module,
    procedures: &'a crate::ir::ProcedureInventory,
    frame_facts: HashMap<crate::ir::ProcedureId, UpvarInfo>,
    outer_facts: HashMap<crate::ir::ProcedureId, GlobalWriteInfo>,
    redefined: &'a HashSet<String>,
    mutations: &'a ModuleCommandMutations,
    projection: &'a ProcBindingTrustProjection,
    source: Option<SourceSummaryContext<'a>>,
    native_source: Option<NativeSummaryContext<'a>>,
    /// The finished summaries, by qualified name.
    summaries: RefCell<HashMap<crate::ir::ProcedureId, TransferSummary>>,
    /// The roles of the procedures of a cycle being solved: `None` while a
    /// procedure is not yet known to complete normally.
    provisional: RefCell<HashMap<crate::ir::ProcedureId, Option<Vec<ParamRole>>>>,
    /// The procedures of a cycle being solved whose component writes no
    /// place outside its frames.
    provisional_closed: RefCell<HashSet<crate::ir::ProcedureId>>,
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
    units: Option<&'a HashMap<crate::ir::ProcedureId, crate::compilation_unit::FunctionUnit>>,
    /// Each procedure's SSA, built or borrowed once for its re-runs.
    ssa: RefCell<HashMap<crate::ir::ProcedureId, Rc<SsaFunction>>>,
    /// Each re-run made: by callee, seeds and whole-module trust.
    reruns: RefCell<HashMap<RerunKey, Option<Rc<Rerun>>>>,
    /// The re-runs under way, innermost last.
    active: RefCell<Vec<RerunKey>>,
    /// How many re-runs were made.
    spent: Cell<u32>,
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
        let frame_effects = crate::cfg_builder::frame_effects::detect_frame_effect_bodies(
            inputs.ir,
            inputs.registry,
        );
        let procedures = Self {
            ir: inputs.ir,
            procedures: &inputs.ir.procedures,
            frame_facts: frame_effects.caller,
            outer_facts: frame_effects.global,
            redefined: &inputs.ir.redefined_procedures,
            mutations: inputs.mutations,
            projection: inputs.projection,
            source: SourceSummaryContext::for_module(inputs.ir, inputs.registry, inputs.config),
            native_source: NativeSummaryContext::for_module(
                inputs.ir,
                inputs.registry,
                inputs.config,
            ),
            summaries: RefCell::new(HashMap::new()),
            provisional: RefCell::new(HashMap::new()),
            provisional_closed: RefCell::new(HashSet::new()),
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
        let frame_effects =
            crate::cfg_builder::frame_effects::detect_frame_effect_bodies(&cu.ir_module, registry);
        Self {
            ir: &cu.ir_module,
            procedures: &cu.ir_module.procedures,
            frame_facts: frame_effects.caller,
            outer_facts: frame_effects.global,
            redefined: &cu.ir_module.redefined_procedures,
            mutations: &cu.command_mutations,
            projection: &cu.caller_scope.proc_binding_trust,
            source: SourceSummaryContext::for_module(
                &cu.ir_module,
                registry,
                cu.ir_module.lexer_config,
            ),
            native_source: NativeSummaryContext::for_module(
                &cu.ir_module,
                registry,
                cu.ir_module.lexer_config,
            ),
            summaries: RefCell::new(cu.transfers.0.clone()),
            provisional: RefCell::new(HashMap::new()),
            provisional_closed: RefCell::new(HashSet::new()),
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

    /// The default `info default` reads for `procedure`'s `parameter`, the
    /// procedure named from `function`: its value, none, or unknown where
    /// the name denotes no procedure of the module whose binding stands, or
    /// the procedure has no such parameter.
    pub(crate) fn parameter_default(
        &self,
        original: Option<&CommandTokens>,
        (procedure, parameter): (&str, &str),
        trust: FoldTrust,
    ) -> ParameterDefault {
        let Some(id) =
            original.and_then(|tokens| self.resolve_original_word(tokens, procedure, trust))
        else {
            return ParameterDefault::Unknown;
        };
        let Some((formals, _)) = self.formals(&id) else {
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
        callee: &crate::ir::ProcedureId,
        words: &[Option<&str>],
    ) -> Option<CallTransfer> {
        let (formals, grammar) = self.formals(callee)?;
        let shape = tcl_syntax::formal_params::formal_argument_count_shape(
            formals.iter().map(|formal| {
                (
                    tcl_syntax::formal_params::is_variadic_formal_name(&formal.name),
                    formal.default.is_some(),
                )
            }),
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
    pub(crate) fn outer_steps(
        &self,
        callee: &crate::ir::ProcedureId,
    ) -> Option<Vec<(String, ExistenceStep)>> {
        if self.provisional_closed.borrow().contains(callee) {
            return Some(Vec::new());
        }
        let summaries = self.summaries.borrow();
        let summary = summaries.get(callee)?;
        // Source coordinates cannot select a current cell or grant Normal.
        // Until that independent application owner exists, retain a barrier.
        summary.global_sources.is_empty().then(Vec::new)
    }

    /// Whether a call to `callee` reaches no place outside its frame: it has
    /// a summary, names no place and writes no outer one.
    #[cfg(test)]
    fn keeps_to_its_frame(&self, callee: &crate::ir::ProcedureId) -> bool {
        self.summaries
            .borrow()
            .get(callee)
            .is_some_and(|summary| summary.links.is_empty() && summary.globals.is_empty())
    }

    /// The parameters of the procedure `qname` names, where it is one of
    /// the module's.
    pub(crate) fn params_of(&self, qname: &crate::ir::ProcedureId) -> Option<&'a [String]> {
        self.source
            .as_ref()?
            .procedure(qname)
            .map(|declared| declared.params.as_slice())
    }

    /// `callee`'s `Name` links: each such parameter's index and the local
    /// its body links to the place it names.
    pub(crate) fn links(&self, callee: &crate::ir::ProcedureId) -> Option<Vec<(usize, String)>> {
        Some(self.summaries.borrow().get(callee)?.links.clone())
    }

    /// The values a call with `arguments` gives `callee`'s parameters —
    /// each argument's where the call supplies it, else the parameter's
    /// default, and the rest of the arguments as one list for a trailing
    /// `args` — each `None` where it is not exact; `None` where the
    /// parameters do not accept the call.
    pub(crate) fn parameter_values(
        &self,
        callee: &crate::ir::ProcedureId,
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

    /// `callee`'s formal parameters, each with the default Tcl binds an
    /// omitted argument to.
    fn formals(
        &self,
        callee: &crate::ir::ProcedureId,
    ) -> Option<(Vec<FormalParameter>, tcl_dialect::ParameterGrammar)> {
        let declared = self.source.as_ref()?.procedure(callee)?;
        crate::registry_invocation::original_procedure_parameters(self.ir, declared, self.registry)
    }
}

/// The step a sequence of outcomes composes to.
fn step_of(outcomes: &[ExistenceOutcome]) -> ExistenceStep {
    outcomes
        .iter()
        .fold(ExistenceStep::PRESERVE, |step, outcome| {
            step.then(ExistenceStep::of(*outcome))
        })
}

/// The revision of a module's summaries: its procedures' names, parameter
/// lists, bodies and redefinitions, and its command bindings.
fn revision_of(ir: &crate::ir::Module, mutations: &ModuleCommandMutations) -> u64 {
    let mut hasher = rustc_hash::FxHasher::default();
    mutations.snapshot().hash(&mut hasher);
    // A procedure's ID is issued per build; its name and declaration are
    // what the source states.
    let mut declared: Vec<&crate::ir::Procedure> = ir.procedures.values().collect();
    declared.sort_unstable_by_key(|procedure| (&procedure.qualified_name, procedure.span.start()));
    for procedure in declared {
        procedure.qualified_name.hash(&mut hasher);
        procedure.span.start().hash(&mut hasher);
        procedure.params_raw.hash(&mut hasher);
        procedure.body_source.hash(&mut hasher);
    }
    let mut redefined: Vec<&String> = ir.redefined_procedures.iter().collect();
    redefined.sort_unstable();
    redefined.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests;
