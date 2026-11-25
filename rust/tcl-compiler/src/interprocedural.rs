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

//! Interprocedural analysis — per-procedure summaries and
//! call-target resolution.
//!
//! Provides the summary types (`ProcSummary`, `MethodSummary`,
//! `InterproceduralAnalysis`) plus the call-target resolver, which
//! plug into the side-effect classifier and the SCCP evaluator.

use std::collections::{HashMap, HashSet};

pub use tcl_registry::Arity;
use tcl_registry::value_transfer::{ExactValue, NumericValue, RepresentationEvidence};

use crate::ir::ProcedureId;
use crate::side_effects::EffectRegion;

mod completion;
mod eager;
mod transfer;

pub(crate) use completion::CompletionWalk;
#[cfg(test)]
pub(crate) use completion::tests::logical_unit as logical_completion_unit;
pub(crate) use eager::DefinitionReach;
pub use transfer::TransferSummaries;
pub(crate) use transfer::{
    CallTransfer, ModuleInputs, ModuleProcedures, NativeValueRefusal, OriginalSummaryExpression,
    OriginalSummaryScript, Rerun, RerunStance,
};

mod body_scan;
mod call_by_name;
mod call_facts;
mod call_targets;
mod exit_values;
mod instance_taint;
mod method_summaries;
mod parameter_observations;
mod positioned_invocations;
mod return_summaries;
mod substitution_calls;
mod summary_fixpoints;

pub(crate) use call_by_name::collect_positioned_call_by_name_reads;
pub use call_by_name::{
    build_proc_index_from_summaries, collect_call_by_name_reads,
    collect_executable_opaque_callee_name_args, collect_opaque_callee_name_args,
    collect_positioned_opaque_callee_name_args,
};
#[cfg(test)]
use call_facts::global_alias_names;
#[cfg(test)]
use call_facts::scan_call_statement;
pub use call_targets::command_prefix_head;
#[cfg(test)]
use call_targets::{namespace_parts_from_proc, resolve_call_target, resolve_internal_call};
use exit_values::seedless_returns;
pub(crate) use exit_values::{ExitBody, ExitReading, exit_value};
use instance_taint::tainted_global_writes_from_cfg;
pub(crate) use instance_taint::{
    enrich_instance_taint_cfg, global_instance_class_source, global_instance_classes,
    global_instance_classes_with, tainted_global_writes,
};
use method_summaries::build_method_summaries;
#[cfg(test)]
use method_summaries::check_ivar_write;
#[cfg(test)]
use method_summaries::collect_instance_var_writes;
#[cfg(test)]
use parameter_observations::{note_params_in_expr, original_word_reference_names};
#[cfg(test)]
pub(crate) use return_summaries::statement_may_return;
#[cfg(test)]
use return_summaries::{classify_return, walk_collect_param_refs};
#[cfg(test)]
use substitution_calls::{scan_expr_for_calls, scan_source_for_calls};
#[cfg(test)]
use summary_fixpoints::finalise_param_traits;
use summary_fixpoints::{
    compute_all_transitive_calls, fixpoint_effects, fixpoint_pure, materialise_summaries,
    scan_all_procs, summary_procedure_bodies,
};

/// Depth cap shared by every `Script`/`Statement`-tree recursion in this
/// module (`collect_instance_var_writes`; the mutually-recursive
/// `scan_script`/`scan_statement`/`scan_control_flow_statement` trio;
/// `script_always_returns`/`stmt_always_returns`).
///
/// Transitively bounded today via `crate::lowering`'s
/// `MAX_LOWER_NEST_DEPTH` (every `Script` this module walks is built by
/// lowering, which already caps its own construction at 256), capped here
/// independently for defence-in-depth and consistency with every other
/// full-tree walker in this crate (`optimiser::MAX_OPTIMISER_WALK_DEPTH`,
/// `codegen::structured::MAX_STRUCTURED_DEPTH`).
const MAX_INTERPROCEDURAL_WALK_DEPTH: tcl_core_types::RecursionLimit =
    tcl_core_types::RecursionLimit(256);

// Summary types

/// A proc/method's declared arity from its bare parameter-name list, as
/// recorded at the IR layer (`Vec<String>`, no default-value info — that
/// lives only in the analyser's richer `ParamDef`; see
/// [`crate::signature_scan::arity::arity_of`] for the default-aware
/// computation used by the arity diagnostics).  Still correctly
/// unbounded when the last parameter is literally `args`, matching
/// [`crate::taint_interproc`]'s identical, already-correct formula for
/// the same bare-name shape.
#[must_use]
pub fn arity_from_names(params: &[String]) -> Arity {
    let n = u16::try_from(params.len()).unwrap_or(u16::MAX);
    if params.last().is_some_and(|p| p == "args") {
        Arity::at_least(n.saturating_sub(1))
    } else {
        Arity::exact(n)
    }
}

/// Interprocedural argument trait. Documents how a parameter is
/// used inside the callee — consumed by the optimiser for
/// parameter-specific reasoning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ProcArgTrait {
    /// Parameter text is substituted into the return value
    /// unchanged.
    Passthrough,
    /// Parameter participates in a comparison that gates control
    /// flow.
    UsedInCondition,
    /// Parameter is forwarded to another procedure.
    ForwardedToCallee,
    /// Parameter names a variable the proc reads via `upvar` (a
    /// read-only caller-frame alias, or a name source) — call-by-name.
    VarRead,
    /// Parameter names a variable the proc writes via `upvar` +
    /// `set` / `incr` / `append` (a caller-frame write-back) — call-by-name.
    VarWrite,
    /// Parameter is never read.
    Unused,
}

impl ProcArgTrait {
    /// Stable lower-case wire form
    /// (`"passthrough"`, `"used_in_condition"`,
    /// `"forwarded_to_callee"`, `"var_read"`, `"var_write"`,
    /// `"unused"`). Consumers materialise traits using this form
    /// rather than re-implementing the mapping.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Passthrough => "passthrough",
            Self::UsedInCondition => "used_in_condition",
            Self::ForwardedToCallee => "forwarded_to_callee",
            Self::VarRead => "var_read",
            Self::VarWrite => "var_write",
            Self::Unused => "unused",
        }
    }
}

/// A procedure's byte-exact return text and its reporting classification.
#[derive(Debug, Clone, PartialEq)]
pub struct ConstantReturn(ExactValue<String>);

impl ConstantReturn {
    /// Classify only spellings that this summary can reproduce byte for byte.
    pub(crate) fn from_exact(value: &ExactValue) -> Option<Self> {
        let text = value.as_str().ok()?;
        let numeric = ExactValue::from_literal(text)
            .numeric
            .or_else(|| {
                let value = text.parse::<f64>().ok().filter(|value| value.is_finite())?;
                (ExactValue::shortest_double(value).bytes == text.as_bytes())
                    .then_some(NumericValue::Float(value))
            })
            .or_else(|| text.parse::<bool>().ok().map(NumericValue::Bool));
        Some(Self::with_reading(text.to_owned(), numeric))
    }

    fn with_reading(text: String, numeric: Option<NumericValue>) -> Self {
        Self(ExactValue {
            bytes: text,
            numeric,
            representation: RepresentationEvidence::Unknown,
        })
    }

    /// An integer with its canonical decimal spelling.
    #[must_use]
    pub fn int(value: i64) -> Self {
        Self::with_reading(value.to_string(), Some(NumericValue::Int(value)))
    }

    /// A double in the shortest spelling used by the summary wire format.
    #[must_use]
    pub fn float(value: f64) -> Self {
        Self::with_reading(
            tcl_syntax::number::format_double(value),
            Some(NumericValue::Float(value)),
        )
    }

    /// A boolean spelled `true` or `false` in the returned text.
    #[must_use]
    pub fn boolean(value: bool) -> Self {
        Self::with_reading(value.to_string(), Some(NumericValue::Bool(value)))
    }

    /// Text without a numeric reading.
    #[must_use]
    pub fn string(value: String) -> Self {
        Self::with_reading(value, None)
    }

    /// The stored return text and its numeric reading.
    #[must_use]
    pub const fn exact(&self) -> &ExactValue<String> {
        &self.0
    }

    /// Lower into the canonical `(kind, text)` wire form. `kind`
    /// is one of `"int"`, `"float"`, `"bool"`, `"str"`; `text` is
    /// [`Self::text`], the value as the procedure returns it.
    #[must_use]
    pub fn as_kind_text(&self) -> (&'static str, String) {
        let kind = match self.0.numeric {
            Some(NumericValue::Int(_)) => "int",
            Some(NumericValue::Float(_)) => "float",
            Some(NumericValue::Bool(_)) => "bool",
            None => "str",
        };
        (kind, self.text())
    }

    /// The value's text, as the procedure returns it: the summary keeps an
    /// integer, a double or a boolean only where this spells the value byte
    /// for byte, so a fold spells the value exactly.
    #[must_use]
    pub fn text(&self) -> String {
        self.0.bytes.clone()
    }
}

/// Per-procedure summary of interprocedural facts.
// False positive: a flat record of independent boolean facts about a
// procedure (barrier / unknown-calls / writes-global / pure / ...), not a
// state machine — there is no natural enum to collapse these into.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq)]
pub struct ProcSummary {
    /// Source reporting label; target selection uses the map's issued body ID.
    pub qualified_name: String,
    /// Parameter names in declaration order.
    pub params: Vec<String>,
    /// Declared arity.
    pub arity: Arity,
    /// Module-issued IDs of transitive callees.
    pub calls: Vec<ProcedureId>,
    /// Exact direct callee IDs, without transitive edges.
    /// Graph consumers render labels only after joining these IDs.
    pub direct_calls: Vec<ProcedureId>,
    /// True if the body contains a barrier command.
    pub has_barrier: bool,
    /// True if the body calls a command not in the registry and
    /// not resolvable to another internal proc.
    pub has_unknown_calls: bool,
    /// True if the body writes any global / namespace variable.
    pub writes_global: bool,
    /// True if the body is side-effect-free.
    pub pure: bool,
    /// Whether a call whose word count the parameters accept completes
    /// normally whatever its arguments hold: the body is straight-line, reads
    /// only scalars it holds set, and runs only commands that complete so — a
    /// registry command declaring the normal completion alone, or a procedure
    /// of the module that completes in turn — and never recurses. A
    /// pure procedure that completes is a value that cannot raise, so an
    /// unused store of its call is dead. False where the summary is built
    /// from the IR alone, which holds no command trust.
    pub completes: bool,
    /// Where the module's load surely runs this procedure's `proc`
    /// statement: a direct statement of the top level, or of a `namespace eval`
    /// body that is one. `None` for a definition that may not have run there
    /// — under a condition, in a loop, in another procedure or another file.
    pub defined_at: Option<u32>,
    /// The procedures this one calls whose definitions surely run before the
    /// load may first run it, sorted: the callees the completion proof
    /// may take as defined where this procedure's calls run.
    pub defined_callees: Vec<ProcedureId>,
    /// Effect regions this proc (or its callees) may read.
    pub effect_reads: EffectRegion,
    /// Effect regions this proc (or its callees) may write.
    pub effect_writes: EffectRegion,
    /// True if every return in the body yields the same constant.
    pub returns_constant: bool,
    /// The constant return value when `returns_constant` is true.
    pub constant_return: Option<ConstantReturn>,
    /// Names of parameters whose value influences the return.
    pub return_depends_on_params: Vec<String>,
    /// When set, the return value is exactly the parameter named.
    pub return_passthrough_param: Option<String>,
    /// Whether this proc is eligible for static constant folding.
    pub can_fold_static_calls: bool,
    /// Per-parameter traits.
    pub param_traits: HashMap<String, HashSet<ProcArgTrait>>,
    /// Each parameter with a default: the value Tcl binds it to in a call
    /// that omits it.
    pub param_defaults: HashMap<String, String>,
}

impl ProcSummary {
    /// Build a default summary with conservative values — useful
    /// for stubbing callees whose bodies haven't been analysed.
    #[must_use]
    pub fn unknown(qualified_name: impl Into<String>) -> Self {
        Self {
            qualified_name: qualified_name.into(),
            params: Vec::new(),
            arity: Arity::any(),
            calls: Vec::new(),
            direct_calls: Vec::new(),
            has_barrier: false,
            has_unknown_calls: true,
            writes_global: true,
            pure: false,
            completes: false,
            defined_at: None,
            defined_callees: Vec::new(),
            effect_reads: EffectRegion::UNKNOWN_STATE,
            effect_writes: EffectRegion::UNKNOWN_STATE,
            returns_constant: false,
            constant_return: None,
            return_depends_on_params: Vec::new(),
            return_passthrough_param: None,
            can_fold_static_calls: false,
            param_traits: HashMap::new(),
            param_defaults: HashMap::new(),
        }
    }
}

/// Extended summary for OO methods with class context.
#[derive(Debug, Clone, PartialEq)]
pub struct MethodSummary {
    /// Base procedure summary fields.
    pub base: ProcSummary,
    /// Name of the containing class.
    pub class_name: String,
    /// Method kind: `"method"` / `"classmethod"` / `"constructor"` /
    /// `"destructor"`.
    pub method_kind: crate::ir::MethodKind,
    /// Instance variables the method reads.
    pub reads_instance_vars: HashSet<String>,
    /// Instance variables the method writes.
    pub writes_instance_vars: HashSet<String>,
    /// Names of methods called via `my method`.
    pub calls_my: Vec<String>,
    /// True if the method calls `next` (MRO chain dispatch).
    pub calls_next: bool,
}

/// Result of running interprocedural analysis on a module.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct InterproceduralAnalysis {
    /// Procedure summaries keyed by their issued Module body IDs.
    pub procedures: HashMap<ProcedureId, ProcSummary>,
    /// Method summaries keyed by their issued Module body IDs.
    pub methods: HashMap<crate::ir::ProcedureId, MethodSummary>,
    /// Conditional object/widget class candidates from original top-level
    /// source setup, keyed by procedure. Eager calls see earlier source
    /// factories; callback-only procedure advice sees direct authored setup.
    /// These labels prove no constructor completion or existing Native object.
    pub global_instance_classes: HashMap<ProcedureId, crate::taint::InstanceClassState>,
    /// Complete source metadata used to produce the conditional receiver map.
    /// Equality of input alone cannot authenticate another Module's candidates.
    pub global_instance_class_input: Option<crate::analyser::ResolvedAnalysisInput>,
    /// Original Module interpretation accompanying the receiver candidates.
    /// This supplies source correspondence, never a current object allocation.
    pub global_instance_class_owner:
        Option<std::sync::Arc<crate::command_binding::RetainedSourceModuleBindings>>,
    /// Global/namespace variables made externally controlled by a procedure's
    /// registry-declared instance-option configuration, closed transitively
    /// over the internal call graph.
    pub tainted_global_writes: HashMap<ProcedureId, HashSet<String>>,
    /// Each procedure's transfer summary: what a call does to its caller's
    /// places. A compilation unit's build computes them.
    pub transfers: TransferSummaries,
}

/// One possible procedure's parameter read/write metadata. It selects no
/// target and proves no Native execution or absence of a hidden caller read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcParameterSummary {
    /// This candidate's ordered parameter names.
    pub params: Vec<String>,
    /// Possible uses of those parameters in this candidate.
    pub param_traits: HashMap<String, HashSet<ProcArgTrait>>,
    /// This candidate's omitted-argument defaults.
    pub param_defaults: HashMap<String, String>,
}

/// A conservative constructed-key index of every possible summary sharing a
/// suffix label. [`collect_call_by_name_reads`] unions protective MAY reads;
/// source binding, opaque effects and store-removal proof remain independent.
pub type ProcIndex = HashMap<String, Vec<ProcParameterSummary>>;

// Summary building (partial)

/// The object-handle → candidate-class map
/// ([`crate::object_types::object_handle_classes`]) supplied to the call-graph
/// pass so a `$g walk … -command cb` instance-method callback resolves to a
/// `direct_calls` edge.  A borrow-only newtype: it gives the public
/// [`build_interprocedural_analysis`] a named parameter instead of a bare
/// `HashMap` (whose default hasher would otherwise trip
/// `clippy::implicit_hasher`).  Use [`ObjectTypeMap::none()`] when no object
/// typing is available (an IR-only caller, or a context that needs no callback
/// edges).
#[derive(Clone, Copy)]
pub struct ObjectTypeCandidates<'a> {
    candidates: &'a HashMap<String, HashSet<String>>,
    unknown_residual: bool,
}

/// Compatibility name for advisory callback reachability candidates.
/// This carrier cannot establish runtime object contents or closed dispatch.
pub type ObjectTypeMap<'a> = ObjectTypeCandidates<'a>;

impl<'a> ObjectTypeCandidates<'a> {
    /// Supply possible classes for additional callback reachability edges.
    #[must_use]
    pub const fn candidates(classes: &'a HashMap<String, HashSet<String>>) -> Self {
        Self {
            candidates: classes,
            unknown_residual: true,
        }
    }

    /// Scope/name unions always leave the actual receiver's contents unresolved.
    #[must_use]
    pub const fn unknown_residual(self) -> bool {
        self.unknown_residual
    }
}

impl ObjectTypeCandidates<'static> {
    /// The empty map — no object-handle typing (no instance-method callback
    /// edges).  Backed by a process-wide empty map so it needs no local.
    #[must_use]
    pub fn none() -> Self {
        ObjectTypeCandidates::candidates(&EMPTY_OBJECT_TYPES)
    }
}

static EMPTY_OBJECT_TYPES: std::sync::LazyLock<HashMap<String, HashSet<String>>> =
    std::sync::LazyLock::new(HashMap::new);

/// Build conservative interprocedural summaries for every
/// procedure in `ir_module`.
///
/// Populates the structural facts downstream passes
/// (propagation, unused-procs) depend on:
///
/// - `qualified_name`, `params`, `arity`.
/// - `calls` — direct callees whose names resolve to another
///   proc in the module, extended with the transitive closure.
/// - `has_barrier` — `Statement::Barrier` or a direct call to
///   `eval`/`uplevel`/`interp eval`/`namespace eval`.
/// - `has_unknown_calls` — any call that is neither a registry
///   command nor a resolvable internal proc.
/// - `writes_global` — any assignment targeting a global or
///   namespace-scoped variable, any call whose side-effect
///   classification writes `EffectRegion::GLOBAL_STATE`, or any
///   transitive callee that does.
/// - `pure` — least fixpoint over local purity ∧ every callee's
///   purity.
/// - `effect_reads` / `effect_writes` — union over the direct
///   side-effects and the transitive closure's.
#[must_use]
pub fn build_interprocedural_analysis(
    ir_module: &crate::ir::Module,
    registry: &tcl_registry::CommandRegistry,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    object_types: ObjectTypeMap<'_>,
    identities: &crate::realm::CommandBindingRealm,
    declared: Option<&tcl_registry::model::DeclaredSurface>,
) -> InterproceduralAnalysis {
    build_interprocedural_analysis_inner(
        ir_module,
        registry,
        dialect,
        object_types,
        identities,
        declared,
        None,
    )
}

/// Build the summaries of a compilation unit's procedures, each pure one's
/// return read from its own seedless lattice ([`seedless_returns`]): the
/// unit holds every procedure's flow graph and SSA and the module's command
/// trust, which [`build_interprocedural_analysis`], from IR alone, has not.
#[must_use]
pub(crate) fn build_interprocedural_analysis_for_unit(
    cu: &crate::compilation_unit::CompilationUnit,
    registry: &tcl_registry::CommandRegistry,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    object_types: ObjectTypeMap<'_>,
    identities: &crate::realm::CommandBindingRealm,
) -> InterproceduralAnalysis {
    build_interprocedural_analysis_inner(
        &cu.ir_module,
        registry,
        dialect,
        object_types,
        identities,
        Some(&cu.declared_commands),
        Some(ModuleUnits {
            cfg: &cu.cfg_module,
            seedless: Some(SeedlessUnits {
                unit: cu,
                procedures: &cu.procedures,
                mutations: &cu.command_mutations,
                transfers: &cu.transfers,
            }),
        }),
    )
}

/// What a summary build reads beyond the IR: the module's flow graphs, and
/// the procedures' analyses a seedless run needs where the caller has them.
#[derive(Clone, Copy)]
struct ModuleUnits<'a> {
    cfg: &'a crate::cfg::CfgModule,
    seedless: Option<SeedlessUnits<'a>>,
}

/// A compilation unit's per-procedure analyses and the module's command
/// trust: what [`seedless_returns`] runs each procedure's lattice over.
#[derive(Clone, Copy)]
struct SeedlessUnits<'a> {
    /// The unit itself, whose procedures a seedless run's calls reach.
    unit: &'a crate::compilation_unit::CompilationUnit,
    procedures: &'a HashMap<crate::ir::ProcedureId, crate::compilation_unit::FunctionUnit>,
    mutations: &'a crate::command_binding::ModuleCommandMutations,
    /// The transfer summaries the unit's build computed.
    transfers: &'a TransferSummaries,
}

fn build_interprocedural_analysis_inner(
    ir_module: &crate::ir::Module,
    registry: &tcl_registry::CommandRegistry,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    object_types: ObjectTypeMap<'_>,
    identities: &crate::realm::CommandBindingRealm,
    declared: Option<&tcl_registry::model::DeclaredSurface>,
    units: Option<ModuleUnits<'_>>,
) -> InterproceduralAnalysis {
    let object_types = object_types.candidates;
    let summary_bodies = summary_procedure_bodies(ir_module, registry);
    let known: HashSet<ProcedureId> = summary_bodies.keys().copied().collect();

    let local = scan_all_procs(
        ir_module,
        SummaryBodyInputs {
            bodies: &summary_bodies,
            known: &known,
            unit: units
                .and_then(|units| units.seedless)
                .map(|units| units.unit),
        },
        registry,
        dialect,
        object_types,
        identities,
        declared,
    );
    let transitive_calls = compute_all_transitive_calls(&known, &local);
    let pure = fixpoint_pure(&local);
    let (effect_reads, effect_writes) = fixpoint_effects(&local);
    let seedless = units
        .and_then(|units| units.seedless)
        .map_or_default(|units| seedless_returns(ir_module, units, &pure, registry, dialect));
    // Where the load may first run each procedure, and where each one's
    // `proc` statement surely runs: one fact for the completion proof and
    // the instance lifecycle proof.
    let reach = DefinitionReach::of(ir_module, registry);
    // Completion reads the registry commands a body runs as the module
    // leaves their bindings, which a unit's build holds and the IR alone
    // does not.
    let completes = units.and_then(|units| units.seedless).map_or_default(|_| {
        completion::procedures_complete(ir_module, registry, &|callee, caller| {
            reach.defined(callee, caller)
        })
    });

    let mut procedures = materialise_summaries(
        ir_module,
        &summary_bodies,
        &local,
        &transitive_calls,
        (&pure, &completes),
        (&effect_reads, &effect_writes),
        &seedless,
    );
    reach.state(&mut procedures);

    // Summarise TclOO method bodies into `MethodSummary` entries
    // (consumed by the O126 `my <method>` purity gate).  Method bodies are not
    // iterated by the call graph and an unresolved `$obj method` self-dispatch
    // already forces the method impure, so instance-method callback typing adds
    // nothing here — method summaries need no object-type map.
    let methods = build_method_summaries(
        ir_module,
        &known,
        registry,
        dialect,
        identities,
        declared,
        ProcFixpoints {
            pure: &pure,
            effect_reads: &effect_reads,
            effect_writes: &effect_writes,
        },
    );

    let global_instance_classes =
        global_instance_classes_with(ir_module, registry, reach.invocations(), &summary_bodies);
    let tainted_global_writes = units.map_or_else(
        || tainted_global_writes(ir_module, registry, &global_instance_classes),
        |units| {
            tainted_global_writes_from_cfg(ir_module, units.cfg, registry, &global_instance_classes)
        },
    );

    // The unit's transfer summaries, each with the return shape its
    // procedure's summary derives.
    let transfers = units
        .and_then(|units| units.seedless)
        .map(|units| {
            units
                .transfers
                .0
                .iter()
                .map(|(qname, transfer)| {
                    let mut transfer = transfer.clone();
                    if let Some(summary) = procedures.get(qname) {
                        transfer.result = return_shape(summary);
                    }
                    (*qname, transfer)
                })
                .collect()
        })
        .map_or_default(TransferSummaries);

    let global_source = global_instance_class_source(ir_module, registry);
    InterproceduralAnalysis {
        procedures,
        methods,
        global_instance_classes,
        global_instance_class_input: global_source.as_ref().map(|(input, _)| input.clone()),
        global_instance_class_owner: global_source.map(|(_, owner)| owner),
        tainted_global_writes,
        transfers,
    }
}

/// The return shape a procedure's summary states: its constant, the
/// parameter it passes through, the parameters its value reads, or a
/// computed value.
fn return_shape(summary: &ProcSummary) -> ReturnKind {
    if let Some(constant) = &summary.constant_return {
        ReturnKind::Literal(ExactValue::from_literal(&constant.text()))
    } else if let Some(param) = &summary.return_passthrough_param {
        ReturnKind::Passthrough(param.clone())
    } else if summary.return_depends_on_params.is_empty() {
        ReturnKind::Other
    } else {
        ReturnKind::UsesParam(summary.return_depends_on_params.clone())
    }
}

/// The three per-procedure fixpoint results a method summary joins against —
/// its callees' purity and effect sets.  Bundled so [`build_method_summaries`]
/// keeps a small signature.
#[derive(Clone, Copy)]
struct ProcFixpoints<'a> {
    pure: &'a HashMap<ProcedureId, bool>,
    effect_reads: &'a HashMap<ProcedureId, EffectRegion>,
    effect_writes: &'a HashMap<ProcedureId, EffectRegion>,
}

#[derive(Clone, Copy)]
struct SummaryBodyInputs<'a> {
    bodies: &'a HashMap<ProcedureId, &'a crate::ir::Procedure>,
    known: &'a HashSet<ProcedureId>,
    unit: Option<&'a crate::compilation_unit::CompilationUnit>,
}

/// One procedure's local-facts scan, bundled so [`scan_proc`] keeps a small
/// signature.
#[derive(Clone, Copy)]
struct ProcScan<'a> {
    proc: &'a crate::ir::Procedure,
    unit: Option<&'a crate::compilation_unit::CompilationUnit>,
    known: &'a HashSet<ProcedureId>,
    registry: &'a tcl_registry::CommandRegistry,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    object_types: &'a HashMap<String, HashSet<String>>,
    identities: &'a crate::realm::CommandBindingRealm,
    /// The document's own command declarations (`# tcl-lsp: stub`), unioned
    /// with the catalogue through [`ScanCtx::surface`].
    declared: Option<&'a tcl_registry::model::DeclaredSurface>,
    source_input: Option<&'a crate::analyser::ResolvedAnalysisInput>,
    source_procedures: &'a crate::ir::ProcedureInventory,
    source_module: &'a crate::ir::Module,
}

/// Per-procedure scratch facts consumed by the summary-building
/// pipeline.
// False positive: a flat record of independent boolean facts gathered while
// scanning a body, not a state machine — no natural enum to collapse into.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone)]
struct LocalFacts {
    direct_calls: HashSet<ProcedureId>,
    has_barrier: bool,
    has_unknown_calls: bool,
    writes_global: bool,
    local_pure: bool,
    effect_reads: EffectRegion,
    effect_writes: EffectRegion,
    /// Collected return-value classifications — one entry per
    /// `Statement::Return` visited in the body (including those
    /// inside nested compound statements).
    returns: Vec<ReturnKind>,
    /// Accumulated trait observations per parameter name. The
    /// final `ProcSummary::param_traits` is built from this
    /// after the body walk completes.
    param_trait_flags: HashMap<String, HashSet<ProcArgTrait>>,
    param_references: HashSet<String>,
    param_observations_incomplete: bool,
    /// Conditional source incidence only; never participates in Native alias,
    /// effects, purity, successful caller templates or physical cell summaries.
    source_parameter_aliases:
        HashMap<String, crate::registry_invocation::OriginalAuthoredFormalAliasObservation>,
    /// Local variable names this proc aliased into global /
    /// enclosing-namespace scope via `global` / `variable` / `upvar
    /// #0`. A later bare `set g` / `incr g` / `append g` to one of
    /// these mutates a variable the *caller* can see, so it counts as
    /// `writes_global` even though the written name is bare.
    global_aliases: HashSet<String>,
    /// Call-by-name upvar aliases: a local variable name (text) → the
    /// parameter whose value named the *caller-frame* variable it
    /// aliases (`upvar 1 $param local` → `local` → `param`).  A later
    /// `set local …` upgrades that param to [`ProcArgTrait::VarWrite`].
    /// Only level-1 upvars populate this (other levels don't write back
    /// to the caller).
    upvar_aliases: HashMap<String, String>,
    /// Whether the body does anything its caller observes besides linking a
    /// local to the place a parameter names one frame up (`upvar 1 $name
    /// v`): [`Self::local_pure`] with those links set aside.
    impure_beyond_links: bool,
    /// Every local the body links to a place outside its frame by `upvar`,
    /// at any level.
    linked_locals: HashSet<String>,
    /// Each call to a procedure of the module, with its argument words where
    /// the call spells them; `None` for a callback, whose words a later
    /// invocation supplies.
    procedure_calls: Vec<(ProcedureId, Option<Vec<String>>)>,
    /// The positions of the parameters whose value names the place one frame
    /// up a local of the body is linked to.
    name_params: Vec<usize>,
}

impl LocalFacts {
    /// Record something the body does that its caller observes.
    fn mark_impure(&mut self) {
        self.local_pure = false;
        self.impure_beyond_links = true;
    }
}

/// One way a procedure returns, as the summary reads it: a `return`, or the
/// fall-through when the body can reach its end.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ReturnKind {
    /// A value every caller gets: the return's literal word through the
    /// exact value ingress, or what the procedure's seedless lattice proves
    /// there.
    Literal(ExactValue),
    /// `return $param` — a passthrough of a known parameter.
    Passthrough(String),
    /// `return [expr {$param}]` or any return that references a
    /// specific parameter but isn't a plain passthrough.
    UsesParam(Vec<String>),
    /// Any other return (dynamic value, command substitution,
    /// etc.).
    Other,
}

impl Default for LocalFacts {
    fn default() -> Self {
        Self {
            direct_calls: HashSet::new(),
            has_barrier: false,
            has_unknown_calls: false,
            writes_global: false,
            local_pure: false,
            effect_reads: EffectRegion::NONE,
            effect_writes: EffectRegion::NONE,
            returns: Vec::new(),
            param_trait_flags: HashMap::new(),
            param_references: HashSet::new(),
            param_observations_incomplete: true,
            source_parameter_aliases: HashMap::new(),
            global_aliases: HashSet::new(),
            upvar_aliases: HashMap::new(),
            impure_beyond_links: true,
            linked_locals: HashSet::new(),
            procedure_calls: Vec::new(),
            name_params: Vec::new(),
        }
    }
}

/// Read-only context shared by the recursive proc/method body scanners
/// ([`scan_script`] / [`scan_statement`] / [`scan_call_facts`] /
/// [`scan_value_substitutions`]). Bundled so the scanners stay within the
/// argument limit.
#[derive(Clone, Copy)]
struct ScanCtx<'a> {
    known: &'a HashSet<ProcedureId>,
    registry: &'a tcl_registry::CommandRegistry,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    params: &'a HashSet<String>,
    /// Reporting candidates for standalone Logical method summaries; source
    /// receiver and callback ownership comes from retained original receipts.
    object_types: &'a HashMap<String, HashSet<String>>,
    /// The original realm supplies positioned bindings and entered-child
    /// inventories. Raw legacy IR has no original site or Native lookup grant.
    identities: &'a crate::realm::CommandBindingRealm,
    /// The document's own command declarations (`# tcl-lsp: stub` blocks and
    /// `<dialect>.tcl.stubs` sidecars). A stub states the same kind of fact a
    /// `CommandSpec` does, so its argument roles reach this scan through the
    /// same query the catalogue's do — [`ScanCtx::surface`] — rather than a
    /// second table beside `registry`. `None` for a caller with no document
    /// (the optimiser's own unit tests).
    declared: Option<&'a tcl_registry::model::DeclaredSurface>,
    /// Substitutions of the current retained invocation were already scanned
    /// from exact source carriers; raw text must not regain declined semantics.
    positioned_substitutions: bool,
    source_input: Option<&'a crate::analyser::ResolvedAnalysisInput>,
    source_procedures: &'a crate::ir::ProcedureInventory,
    source_module: &'a crate::ir::Module,
    current_tokens: Option<&'a crate::ir::CommandTokens>,
    parameter_body: Option<ProcedureId>,
}

impl<'a> ScanCtx<'a> {
    fn metadata_context(&self) -> Option<std::sync::Arc<tcl_registry::model::ContextRegistry>> {
        self.source_input
            .map(crate::analyser::ResolvedAnalysisInput::context_registry)
    }

    /// The command surface this scan resolves argument roles against: the
    /// catalogue generation plus the document's own declarations.
    fn surface(&self) -> tcl_registry::model::DocumentCommandSurface<'a> {
        tcl_registry::model::DocumentCommandSurface::new(self.registry, self.declared)
    }
}

#[cfg(any(test, feature = "test-support"))]
#[path = "test_body_identity.rs"]
#[doc(hidden)]
pub mod test_body_identity;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod effect_propagation_tests;

#[cfg(test)]
mod selected_global_option_taint_tests;
