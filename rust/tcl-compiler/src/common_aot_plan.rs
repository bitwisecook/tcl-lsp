// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Target-neutral proof plans for procedure calls and materialisable SSA slots.
//!
//! This module consumes existing compiler lattices. It does not lower target
//! instructions and it does not recognise Tcl commands by spelling. Procedure
//! identities come from the lowered module, command mutation and trace hazards
//! come from their shared analyses, and formal-list semantics come from the
//! shared strict Tcl parameter parser.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use tcl_registry::model::semantic::SemanticContext;
use tcl_registry::{DispatchDependencies, DispatchDependencyDomain, Traits};
use tcl_registry::{SemanticOperationId, hooks::LoweringHookId};

use crate::analyses::LatticeValue;
use crate::cfg::{Block, BlockId};
use crate::command_binding::{BindingKind, CommandBinding, analyse_command_binding};
use crate::compilation_unit::{CompilationUnit, FunctionUnit};
use crate::intervals::{Interval, compute_intervals_with, numbers_for_dialect};
use crate::ir::{CommandTokens, Procedure, Statement};
use crate::registry_invocation::{RegistryInvocationResolution, resolve_command_tokens};
use crate::representation_plan::{SharingState, VarStorage};
use crate::semantic_optimisation::{SemanticOptimisationConfig, SemanticOptimisationPassId};
use crate::ssa::{SsaBlock, SsaStatement, Symbol, ValueKey};
use crate::types::{TypeKind, TypeLattice, TypeShape, type_join};
use crate::var_escape::{EscapeTag, ProcEscapeSummary, analyse_var_escape_cu};

/// Stable identity of one CFG invocation, including an immediate command
/// substitution nested in one argument of the enclosing statement.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DirectCallSiteId {
    /// Qualified caller function.
    pub function: String,
    /// CFG block identity.
    pub block: BlockId,
    /// Statement position in the block.
    pub statement_index: u32,
    /// `None` for the statement call itself, or the argument index containing
    /// the bracketed call.
    pub nested_argument: Option<u32>,
}

/// Stable source identity of an in-unit Tcl procedure definition.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProcedureIdentity {
    /// Fully qualified Tcl command name.
    pub qualified_name: String,
    /// Definition source-span start.
    pub definition_start: u32,
    /// Definition source-span end.
    pub definition_end: u32,
}

/// Stable identity of one SSA value in one procedure.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SsaValueIdentity {
    /// Qualified procedure name.
    pub function: String,
    /// Function-local interned variable symbol.
    pub symbol: Symbol,
    /// SSA version.
    pub version: u32,
}

/// Whether compiled code is hosted by a live interpreter or owns the whole
/// Tcl program and its final state.
///
/// This is a proof premise, not a backend option. Top-level variables remain
/// observable after `::top` returns in [`Self::Hosted`], so that environment
/// can never infer native-only storage merely from local type information.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommonAotEnvironment {
    /// A caller may inspect or mutate interpreter state after compiled code.
    Hosted,
    /// The compiled program owns the interpreter lifetime and final state.
    SealedProgram,
}

impl CommonAotEnvironment {
    /// Whether this environment can participate in a later native-only proof.
    /// Escape, trace, representation, and boundary proofs remain mandatory.
    #[must_use]
    pub const fn permits_native_only(self) -> bool {
        matches!(self, Self::SealedProgram)
    }
}

/// Selected direct-procedure proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectProcEvidence {
    /// Exact in-unit definition selected by command-binding analysis.
    pub callee: ProcedureIdentity,
    /// Fixed required formal names, in Tcl binding order.
    pub formals: Vec<String>,
    /// Types propagated from this call's already-evaluated actual arguments.
    pub actual_types: Vec<TypeLattice>,
    /// Exact caller values corresponding to each formal, when retained by SSA.
    pub actual_values: Vec<DirectActualValue>,
    /// Exact environment premise used for Tcl numeric and completion rules.
    pub context: Option<SemanticContext>,
    /// Mutable dispatch domains a later runtime guard must cover.
    pub dispatch_dependencies: DispatchDependencies,
    /// Whether the entire body is authorised for specialised execution.
    pub body: DirectProcBodyDecision,
    /// Existing escape analysis proved that the body does not expose its frame.
    /// This is necessary but not sufficient for frame elimination.
    pub frame_escape_private: bool,
    /// Whether the procedure frame may be omitted. This can become `true`
    /// only when `body` is selected as well as the independent frame pass.
    pub frame_elidable: bool,
}

/// Caller-side identity of one already-evaluated direct-proc actual.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DirectActualValue {
    /// A whole-word scalar variable use with an exact SSA identity.
    Ssa(SsaValueIdentity),
    /// Compatibility lowering did not retain a value identity precise enough
    /// for a backend to prove materialisation.
    Unproven,
}

/// Selection or typed decline for specialisation inside a direct proc body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DirectProcBodyDecision {
    /// Registry identities and every mutable dependency of the body survived
    /// lowering and are covered by the proof.
    Selected(DirectProcBodyEvidence),
    /// Direct dispatch may still be useful, but the body must execute through
    /// its general Tcl semantics.
    Declined(DirectProcBodyDecline),
}

/// Registry-derived dispatch obligations for a specialised proc body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectProcBodyEvidence {
    /// Registry-owned operations whose live bindings were proved trustworthy.
    pub operations: Vec<SemanticOperationId>,
    /// Union of mutable domains required by every operation in the body.
    pub dispatch_dependencies: DispatchDependencies,
}

/// Why the body of a directly resolved proc cannot yet be specialised.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirectProcBodyDecline {
    /// The independent frame-elision pass was not explicitly enabled.
    FrameElisionPassDisabled,
    /// The independent integer-specialisation pass was not enabled.
    NativeIntegerPassDisabled,
    /// The body is outside the deliberately narrow initial closed-expression
    /// tier. It may still execute through the general Tcl body path.
    UnsupportedBodyShape,
    /// Compatibility lowering erased one or more registry command identities,
    /// so rebound `expr`, `return`, traces, and equivalent dialect hooks cannot
    /// yet be proved or guarded generically.
    InternalDispatchProofUnavailable,
    /// One registry-owned operation was rebound or lacks a registry spelling.
    InternalDispatchUntrusted {
        /// Operation whose binding could not be proved.
        operation: SemanticOperationId,
    },
    /// An execution trace can observe an operation inside the proc body.
    InternalExecutionTrace {
        /// Operation with a traced registry spelling.
        operation: SemanticOperationId,
    },
}

/// Whether every registry spelling of one semantic operation retains its
/// declared binding in this module.
///
/// This target-neutral query is shared by common proof construction and legacy
/// backends. Consumers never name Tcl commands themselves.
#[must_use]
pub fn semantic_operation_binding_is_trusted(
    registry: &tcl_registry::CommandRegistry,
    mutations: &crate::command_binding::ModuleCommandMutations,
    operation: SemanticOperationId,
) -> bool {
    let mut commands = registry
        .command_names_for_semantic_operation(operation)
        .peekable();
    commands.peek().is_some() && commands.all(|command| mutations.trusts(command))
}

/// Why common analysis did not select a direct procedure call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DirectProcDecline {
    /// The pass is disabled by default and was not explicitly enabled.
    PassDisabled,
    /// No resolved environment was carried by the unit's semantic bundle.
    ContextUnavailable,
    /// Flow-sensitive command binding did not name the original procedure.
    BindingNotProcedure {
        /// Flow-sensitive binding class observed at the call site.
        kind: BindingKind,
    },
    /// The name was redirected through an alias or rename.
    ReboundOrAliased,
    /// A dynamic command-table mutation can invalidate every name.
    DynamicCommandMutation,
    /// An execution trace with a dynamic target can observe any call.
    DynamicExecutionTrace,
    /// This procedure has an execution trace, including step traces.
    ExecutionTrace,
    /// Tcl's strict formal-list parser rejected the retained declaration.
    InvalidFormalList,
    /// Defaulted parameters are deliberately outside this first direct tier.
    DefaultArgumentUnsupported,
    /// A trailing `args` formal is deliberately outside this first direct tier.
    VariadicUnsupported,
    /// The already-evaluated argv does not match the fixed formal count.
    ArityMismatch {
        /// Number of fixed required formals.
        expected: usize,
        /// Number of already-evaluated actual arguments.
        actual: usize,
    },
    /// Escape/call analysis found a dynamic frame or nested fallback surface.
    DynamicCallee,
    /// Current completion planning cannot yet rewrite a CFG with exceptional edges.
    ExceptionalControlFlow,
}

impl DirectProcDecline {
    /// Stable Explorer/API spelling.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::PassDisabled => "pass-disabled",
            Self::ContextUnavailable => "context-unavailable",
            Self::BindingNotProcedure { .. } => "binding-not-procedure",
            Self::ReboundOrAliased => "rebound-or-aliased",
            Self::DynamicCommandMutation => "dynamic-command-mutation",
            Self::DynamicExecutionTrace => "dynamic-execution-trace",
            Self::ExecutionTrace => "execution-trace",
            Self::InvalidFormalList => "invalid-formal-list",
            Self::DefaultArgumentUnsupported => "default-argument-unsupported",
            Self::VariadicUnsupported => "variadic-unsupported",
            Self::ArityMismatch { .. } => "arity-mismatch",
            Self::DynamicCallee => "dynamic-callee",
            Self::ExceptionalControlFlow => "exceptional-control-flow",
        }
    }
}

/// Selection or typed decline for one direct-call candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DirectProcDecision {
    /// A target-neutral direct call is authorised.
    Selected(DirectProcEvidence),
    /// The generic Tcl invocation remains required.
    Declined(DirectProcDecline),
}

/// Mapping from an outer registry invocation argument to its evaluated source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SemanticCallArgument {
    /// One ordinary already-evaluated outer argument.
    Value {
        /// Zero-based index after the command head.
        outer_argument: u32,
    },
    /// This outer argument is exactly the result of a selected nested direct
    /// call; substitutions are evaluated once and are never replayed.
    NestedDirect {
        /// Zero-based index after the command head.
        outer_argument: u32,
        /// Exact selected nested call site.
        call: DirectCallSiteId,
    },
}

/// Stable registry form identity selected from structured invocation words.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SemanticFormIdentity(String);

impl SemanticFormIdentity {
    /// Stable registry form spelling.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Registry-resolved, binding-proved semantic call evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticCallEvidence {
    /// Registry-owned operation identity selected from retained source words.
    pub operation: SemanticOperationId,
    /// Registry form identity, when a determinate form matched.
    pub form: Option<SemanticFormIdentity>,
    /// Environment premise used for registry resolution.
    pub context: Option<SemanticContext>,
    /// Mutable interpreter domains a runtime guard must cover.
    pub dispatch_dependencies: DispatchDependencies,
    /// Registry-declared behaviour traits used by boundary coverage proofs.
    pub traits: Traits,
    /// Exact outer-to-nested argument relationship.
    pub arguments: Vec<SemanticCallArgument>,
}

/// Why an outer call could not receive semantic-operation evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SemanticCallDecline {
    /// The pass is independently disabled by default.
    PassDisabled,
    /// No resolved environment was carried by the unit's semantic bundle.
    ContextUnavailable,
    /// Compatibility lowering did not retain structured invocation words.
    TokensUnavailable,
    /// Structured registry resolution could not select an exact operation/form.
    RegistryUnresolved,
    /// The operation is the generic invocation fallback, not a specialisation.
    GenericInvocation,
    /// Flow-sensitive binding no longer names the original registry command.
    BindingNotOriginal {
        /// Flow-sensitive binding class observed at the call site.
        kind: BindingKind,
    },
    /// A dynamic command-table mutation invalidates all static bindings.
    DynamicCommandMutation,
    /// Whole-module mutation analysis found a possible rebinding.
    ReboundOrAliased,
    /// A dynamic execution trace can observe any call or step.
    DynamicExecutionTrace,
    /// An execution trace, including step traces, observes this command.
    ExecutionTrace,
    /// A nested substitution exists but its exact direct call was not selected.
    NestedDirectUnavailable {
        /// Zero-based outer argument containing the substitution.
        outer_argument: u32,
    },
}

/// Selection or typed decline for one outer semantic call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SemanticCallDecision {
    /// Common analysis proved registry semantics and live binding identity.
    Selected(SemanticCallEvidence),
    /// The call remains a general Tcl boundary.
    Declined(SemanticCallDecline),
}

/// One materialisable SSA slot authorised by common analysis.
#[derive(Debug, Clone, PartialEq)]
pub struct MaterialisableSlotEvidence {
    /// Existing local-slot allocation reused by both future backends.
    pub local_slot: u32,
    /// Variable display name retained for diagnostics.
    pub variable: String,
    /// Exact singleton type shape used by materialisation.
    pub shape: TypeShape,
    /// Existing sparse-conditional-constant-propagation fact.
    pub constant: Option<LatticeValue>,
    /// Existing integer-range fact, when meaningful.
    pub interval: Option<Interval>,
    /// Authoritative storage selected by this common proof.
    pub storage: VarStorage,
    /// Exact recipe for reconstructing the runtime-visible Tcl value.
    pub recipe: MaterialisationRecipe,
    /// Live-interpreter epochs a later consumer must guard before using this
    /// slot without a continuously authoritative frame cell.
    pub runtime_guards: MaterialisationGuardRequirements,
}

/// Runtime observability domains not discharged by source-only analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaterialisationGuardRequirements {
    /// Guard the interpreter's variable-trace generation.
    pub variable_trace_epoch: bool,
    /// Guard safe/child-interpreter policy and host callbacks that can expose
    /// frames independently of source-visible commands.
    pub interpreter_policy_epoch: bool,
}

/// How a materialisable slot recreates its runtime-visible Tcl value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaterialisationRecipe {
    /// Retain the original Tcl object. Publishing that same object preserves
    /// non-canonical string reps such as `02`, `+1`, and `0x10`; ordinary
    /// shared-object copy-on-write rules apply to later mutation.
    RetainOriginalTclObject {
        /// Ownership state the eventual lowering must preserve.
        sharing: SharingState,
    },
}

/// Why an SSA value cannot use a materialisable slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MaterialisableSlotDecline {
    /// The pass is disabled by default and was not explicitly enabled.
    PassDisabled,
    /// Escape analysis did not allocate a local slot for this name.
    NoLocalSlot,
    /// This SSA version may be observed through a frame alias.
    EscapesToFrame,
    /// A literal or dynamic variable trace may observe representation changes.
    VariableTrace,
    /// Type tracking did not prove one exact representation shape.
    TypeNotSingleton,
    /// Exceptional CFG edges are not yet covered by slot materialisation.
    ExceptionalControlFlow,
    /// A hosted top-level variable remains observable after `::top` returns.
    HostedTopLevelObservable,
}

impl MaterialisableSlotDecline {
    /// Stable Explorer/API spelling.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::PassDisabled => "pass-disabled",
            Self::NoLocalSlot => "no-local-slot",
            Self::EscapesToFrame => "escapes-to-frame",
            Self::VariableTrace => "variable-trace",
            Self::TypeNotSingleton => "type-not-singleton",
            Self::ExceptionalControlFlow => "exceptional-control-flow",
            Self::HostedTopLevelObservable => "hosted-top-level-observable",
        }
    }
}

/// Selection or typed decline for one SSA slot candidate.
#[derive(Debug, Clone, PartialEq)]
pub enum MaterialisableSlotDecision {
    /// Common analysis proved an exact materialisation recipe.
    Selected(MaterialisableSlotEvidence),
    /// The value stays boxed/frame-resident.
    Declined(MaterialisableSlotDecline),
}

/// Coverage deliberately excluded from this initial common tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommonAotCoverageDecline {
    /// `TclOO` method dispatch requires its own guarded method identity.
    TclOoMethods,
    /// `apply` and namespace-eval body units do not denote direct proc commands.
    SyntheticBodyUnits,
}

/// Stable identity of one top-level CFG statement.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CfgStatementId {
    /// Qualified function name.
    pub function: String,
    /// CFG block containing the statement.
    pub block: BlockId,
    /// Statement ordinal within the block.
    pub statement_index: u32,
}

/// How one statement is accounted for by an exact sealed-program proof.
#[derive(Debug, Clone, PartialEq)]
pub enum ClosedProgramStatementEvidence {
    /// A procedure definition whose exact definition is the target of a
    /// selected direct call.
    DirectProcedureDefinition {
        /// Statement identity.
        statement: CfgStatementId,
        /// Selected procedure definition.
        procedure: ProcedureIdentity,
    },
    /// A constant definition whose SSA value is an exact selected direct-call
    /// actual.
    DirectActualConstant {
        /// Statement identity.
        statement: CfgStatementId,
        /// Exact caller SSA value.
        value: SsaValueIdentity,
        /// Exact SCCP value.
        constant: LatticeValue,
        /// Registry operation whose binding and trace proof authorises
        /// eliding the source assignment command.
        operation: SemanticOperationId,
    },
    /// A registry-resolved frameless boundary whose arguments are selected
    /// nested direct calls.
    SemanticBoundary {
        /// Exact outer call site.
        call: DirectCallSiteId,
        /// Registry operation executed at the boundary.
        operation: SemanticOperationId,
    },
}

/// Exact accounting of every statement in one closed top-level program.
#[derive(Debug, Clone, PartialEq)]
pub struct ClosedProgramCoverageEvidence {
    /// Sole reachable top-level block.
    pub block: BlockId,
    /// Every top-level statement, in source/CFG order.
    pub statements: Vec<ClosedProgramStatementEvidence>,
}

/// Why exact sealed-program statement accounting was unavailable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClosedProgramCoverageDecline {
    /// Hosted compilation preserves externally observable interpreter state.
    HostedEnvironment,
    /// The top-level CFG is not one closed linear block with normal completion.
    NonLinearControlFlow,
    /// At least one statement lacks one of the exact common proof categories.
    UncoveredStatement,
}

/// Exact coverage evidence or a conservative typed decline.
#[derive(Debug, Clone, PartialEq)]
pub enum ClosedProgramCoverageDecision {
    /// Every top-level statement is explicitly accounted for.
    Selected(ClosedProgramCoverageEvidence),
    /// A backend must retain general top-level execution.
    Declined(ClosedProgramCoverageDecline),
}

/// Target-neutral AOT proof evidence keyed by stable compiler IR identities.
#[derive(Debug, Clone, PartialEq)]
pub struct CommonAotProofPlan {
    environment: CommonAotEnvironment,
    direct_calls: BTreeMap<DirectCallSiteId, DirectProcDecision>,
    declined_direct_callees: BTreeSet<String>,
    semantic_calls: BTreeMap<DirectCallSiteId, SemanticCallDecision>,
    closed_program_coverage: ClosedProgramCoverageDecision,
    materialisable_slots: BTreeMap<SsaValueIdentity, MaterialisableSlotDecision>,
    coverage_declines: Vec<CommonAotCoverageDecline>,
}

impl CommonAotProofPlan {
    /// Build proof evidence from existing common analyses.
    #[must_use]
    pub fn build(
        unit: &CompilationUnit,
        registry: &tcl_registry::CommandRegistry,
        context: Option<SemanticContext>,
        config: SemanticOptimisationConfig,
        environment: CommonAotEnvironment,
    ) -> Self {
        let escape = analyse_var_escape_cu(unit, true);
        let mutations =
            crate::command_binding::scan_module_command_mutations(&unit.ir_module, registry);
        let direct = collect_direct_calls(unit, registry, context, config, &escape, &mutations);
        let direct_calls = direct.decisions;
        let declined_direct_callees = direct.declined_callees;
        let propagated = direct.propagated;

        let semantic_calls =
            collect_semantic_calls(unit, registry, context, config, &mutations, &direct_calls);

        let closed_program_coverage = prove_closed_program_coverage(
            unit,
            registry,
            environment,
            &mutations,
            &direct_calls,
            &semantic_calls,
        );

        let materialisable_slots = collect_materialisable_slots(
            unit,
            &escape,
            &propagated,
            config,
            environment,
            &closed_program_coverage,
        );

        let mut coverage_declines = Vec::new();
        if !unit.methods.is_empty() {
            coverage_declines.push(CommonAotCoverageDecline::TclOoMethods);
        }
        if !unit.body_units.is_empty() {
            coverage_declines.push(CommonAotCoverageDecline::SyntheticBodyUnits);
        }
        Self {
            environment,
            direct_calls,
            declined_direct_callees,
            semantic_calls,
            closed_program_coverage,
            materialisable_slots,
            coverage_declines,
        }
    }

    /// Compilation-environment premise under which this proof was built.
    #[must_use]
    pub const fn environment(&self) -> CommonAotEnvironment {
        self.environment
    }

    /// Direct-call decisions in stable IR order.
    pub fn direct_calls(&self) -> impl Iterator<Item = (&DirectCallSiteId, &DirectProcDecision)> {
        self.direct_calls.iter()
    }

    /// Whether any direct-call candidate naming `callee` was declined.
    ///
    /// A selected call site proves nothing about the *other* ways a procedure
    /// can be reached: a same-arity call through an alias, a rebound name, or
    /// a candidate declined for exceptional control flow still invokes it with
    /// values the selected sites never carry.  A consumer that joins facts
    /// across selected sites — caller operand ranges, propagated actual types —
    /// must therefore treat a decline naming the same callee as proof that the
    /// selected sites do not exhaust its callers.
    #[must_use]
    pub fn has_declined_direct_call_to(&self, callee: &str) -> bool {
        self.declined_direct_callees.contains(callee)
    }

    /// Registry semantic-call decisions in stable CFG identity order.
    pub fn semantic_calls(
        &self,
    ) -> impl Iterator<Item = (&DirectCallSiteId, &SemanticCallDecision)> {
        self.semantic_calls.iter()
    }

    /// Return the semantic-call decision for one exact outer call site.
    #[must_use]
    pub fn semantic_call(&self, site: &DirectCallSiteId) -> Option<&SemanticCallDecision> {
        self.semantic_calls.get(site)
    }

    /// Exact sealed top-level statement accounting, when proved.
    #[must_use]
    pub const fn closed_program_coverage(&self) -> &ClosedProgramCoverageDecision {
        &self.closed_program_coverage
    }

    /// Materialisable-slot decisions in stable SSA order.
    pub fn materialisable_slots(
        &self,
    ) -> impl Iterator<Item = (&SsaValueIdentity, &MaterialisableSlotDecision)> {
        self.materialisable_slots.iter()
    }

    /// Deliberately excluded semantic surfaces.
    #[must_use]
    pub fn coverage_declines(&self) -> &[CommonAotCoverageDecline] {
        &self.coverage_declines
    }
}

struct CallCandidate<'a> {
    id: DirectCallSiteId,
    block: BlockId,
    statement_index: u32,
    command: String,
    args: Vec<String>,
    tokens: Option<&'a CommandTokens>,
    ssa_statement: &'a crate::ssa::SsaStatement,
}

fn function_units(unit: &CompilationUnit) -> impl Iterator<Item = (&str, &FunctionUnit)> {
    std::iter::once((unit.top_level.name.as_str(), &unit.top_level)).chain(
        unit.procedures
            .iter()
            .map(|(name, unit)| (name.as_str(), unit)),
    )
}

fn call_sites<'a>(caller: &str, function: &'a FunctionUnit) -> Vec<CallCandidate<'a>> {
    let mut out = Vec::new();
    for (block, cfg_block) in &function.cfg.blocks {
        let Some(ssa_block) = function.ssa.blocks.get(block) else {
            continue;
        };
        for (index, statement) in cfg_block.statements.iter().enumerate() {
            let Some(ssa_statement) = ssa_block.statements.get(index) else {
                continue;
            };
            let Statement::Call {
                command,
                args,
                tokens,
                ..
            } = statement
            else {
                continue;
            };
            let statement_index = u32::try_from(index).unwrap_or(u32::MAX);
            out.push(CallCandidate {
                id: DirectCallSiteId {
                    function: caller.to_owned(),
                    block: *block,
                    statement_index,
                    nested_argument: None,
                },
                block: *block,
                statement_index,
                command: command.clone(),
                args: args.clone(),
                tokens: tokens.as_ref(),
                ssa_statement,
            });
            for (argument_index, argument) in args.iter().enumerate() {
                if let Some((nested, nested_args)) =
                    crate::value_shapes::parse_command_substitution(argument)
                {
                    out.push(CallCandidate {
                        id: DirectCallSiteId {
                            function: caller.to_owned(),
                            block: *block,
                            statement_index,
                            nested_argument: Some(
                                u32::try_from(argument_index).unwrap_or(u32::MAX),
                            ),
                        },
                        block: *block,
                        statement_index,
                        command: nested,
                        args: nested_args,
                        tokens: None,
                        ssa_statement,
                    });
                }
            }
        }
    }
    out
}

struct DirectCollection {
    decisions: BTreeMap<DirectCallSiteId, DirectProcDecision>,
    /// Callees named by at least one *declined* candidate — the set a consumer
    /// joining facts across selected sites must consult before treating those
    /// sites as the callee's complete caller set.
    declined_callees: BTreeSet<String>,
    propagated: HashMap<(String, usize), Option<TypeLattice>>,
}

fn collect_direct_calls(
    unit: &CompilationUnit,
    registry: &tcl_registry::CommandRegistry,
    context: Option<SemanticContext>,
    config: SemanticOptimisationConfig,
    escape: &HashMap<String, ProcEscapeSummary>,
    mutations: &crate::command_binding::ModuleCommandMutations,
) -> DirectCollection {
    let known: HashSet<String> = unit.ir_module.procedures.keys().cloned().collect();
    let mut direct_calls = BTreeMap::new();
    let mut declined_callees = BTreeSet::new();
    // `None` is a poison fact: at least one selected call supplied an actual
    // whose type was not proved, or the callee has an unselected caller (see
    // the declined-callee sweep below). `TypeLattice::Unknown` is lattice
    // bottom, so joining it directly would incorrectly preserve another call's
    // fact.
    let mut propagated: HashMap<(String, usize), Option<TypeLattice>> = HashMap::new();
    for (caller_name, function) in function_units(unit) {
        let bindings = analyse_command_binding(&function.cfg, registry, &[]);
        for site in call_sites(caller_name, function) {
            let binding =
                bindings.binding_at(site.block, site.statement_index as usize, &site.command);
            let resolved =
                crate::interprocedural::resolve_internal_call(&site.command, caller_name, &known)
                    .or_else(|| {
                        binding
                            .target
                            .clone()
                            .filter(|target| known.contains(target))
                    });
            let Some(callee_name) = resolved else {
                continue;
            };
            let Some(proc_def) = unit.ir_module.procedures.get(&callee_name) else {
                continue;
            };
            let decision = direct_decision(&DirectInputs {
                unit,
                registry,
                function,
                site: &site,
                binding_kind: binding.kind,
                binding_target: binding.target.as_deref(),
                callee_name: &callee_name,
                proc_def,
                summary: escape.get(&callee_name),
                mutations,
                context,
                enabled: config.is_enabled(SemanticOptimisationPassId::DirectProc),
                frame_elision_enabled: config.is_enabled(SemanticOptimisationPassId::FrameElision),
                native_integer_enabled: config
                    .is_enabled(SemanticOptimisationPassId::NativeInteger),
            });
            propagate_actual_types(&mut propagated, &callee_name, &decision);
            if matches!(decision, DirectProcDecision::Declined(_)) {
                declined_callees.insert(callee_name);
            }
            direct_calls.insert(site.id, decision);
        }
    }
    // A declined candidate still calls its callee, with actuals no selected
    // site carries.  Propagated facts are a join over the selected sites only,
    // so for such a callee they describe a caller set that is not the whole
    // one — the same poison state as a selected call whose actual type was
    // never proved.
    for (key, fact) in &mut propagated {
        if declined_callees.contains(&key.0) {
            *fact = None;
        }
    }
    DirectCollection {
        decisions: direct_calls,
        declined_callees,
        propagated,
    }
}

fn propagate_actual_types(
    propagated: &mut HashMap<(String, usize), Option<TypeLattice>>,
    callee_name: &str,
    decision: &DirectProcDecision,
) {
    let DirectProcDecision::Selected(evidence) = decision else {
        return;
    };
    for (index, ty) in evidence.actual_types.iter().enumerate() {
        propagated
            .entry((callee_name.to_owned(), index))
            .and_modify(|current| match current {
                Some(existing) if matches!(ty.kind(), TypeKind::Known | TypeKind::Shimmered) => {
                    *existing = type_join(existing, ty);
                }
                _ => *current = None,
            })
            .or_insert_with(|| {
                matches!(ty.kind(), TypeKind::Known | TypeKind::Shimmered).then(|| ty.clone())
            });
    }
}

fn collect_semantic_calls(
    unit: &CompilationUnit,
    registry: &tcl_registry::CommandRegistry,
    context: Option<SemanticContext>,
    config: SemanticOptimisationConfig,
    mutations: &crate::command_binding::ModuleCommandMutations,
    direct_calls: &BTreeMap<DirectCallSiteId, DirectProcDecision>,
) -> BTreeMap<DirectCallSiteId, SemanticCallDecision> {
    let mut decisions = BTreeMap::new();
    for (caller, function) in function_units(unit) {
        let bindings = analyse_command_binding(&function.cfg, registry, &[]);
        for site in call_sites(caller, function) {
            if site.id.nested_argument.is_some() {
                continue;
            }
            let decision = semantic_call_decision(&SemanticCallInputs {
                unit,
                registry,
                context,
                site: &site,
                bindings: &bindings,
                mutations,
                direct_calls,
                enabled: config
                    .is_enabled(SemanticOptimisationPassId::SemanticOperationSpecialisation),
            });
            decisions.insert(site.id, decision);
        }
    }
    decisions
}

struct SemanticCallInputs<'a> {
    unit: &'a CompilationUnit,
    registry: &'a tcl_registry::CommandRegistry,
    context: Option<SemanticContext>,
    site: &'a CallCandidate<'a>,
    bindings: &'a crate::command_binding::CommandBinding<'a>,
    mutations: &'a crate::command_binding::ModuleCommandMutations,
    direct_calls: &'a BTreeMap<DirectCallSiteId, DirectProcDecision>,
    enabled: bool,
}

fn semantic_call_decision(input: &SemanticCallInputs<'_>) -> SemanticCallDecision {
    let decline = |reason| SemanticCallDecision::Declined(reason);
    if !input.enabled {
        return decline(SemanticCallDecline::PassDisabled);
    }
    if input.context.is_none() {
        return decline(SemanticCallDecline::ContextUnavailable);
    }
    if input.mutations.has_dynamic_mutation() {
        return decline(SemanticCallDecline::DynamicCommandMutation);
    }
    let Some(tokens) = input.site.tokens else {
        return decline(SemanticCallDecline::TokensUnavailable);
    };
    let Ok(RegistryInvocationResolution::Resolved(facts)) =
        resolve_command_tokens(input.registry, input.context, tokens)
    else {
        return decline(SemanticCallDecline::RegistryUnresolved);
    };
    if facts.operation == SemanticOperationId::Invoke {
        return decline(SemanticCallDecline::GenericInvocation);
    }
    let binding = input.bindings.binding_at(
        input.site.block,
        input.site.statement_index as usize,
        &facts.canonical_command,
    );
    if !binding.is_original_builtin() {
        return decline(SemanticCallDecline::BindingNotOriginal { kind: binding.kind });
    }
    if !input.mutations.trusts(&facts.canonical_command) {
        return decline(SemanticCallDecline::ReboundOrAliased);
    }
    if input.unit.ir_module.has_dynamic_trace {
        return decline(SemanticCallDecline::DynamicExecutionTrace);
    }
    if command_has_execution_trace(&input.unit.ir_module, &facts.canonical_command) {
        return decline(SemanticCallDecline::ExecutionTrace);
    }
    let mut arguments = Vec::with_capacity(input.site.args.len());
    for (index, argument) in input.site.args.iter().enumerate() {
        let outer_argument = u32::try_from(index).unwrap_or(u32::MAX);
        if crate::value_shapes::parse_command_substitution(argument).is_some() {
            let nested = DirectCallSiteId {
                function: input.site.id.function.clone(),
                block: input.site.block,
                statement_index: input.site.statement_index,
                nested_argument: Some(outer_argument),
            };
            if !matches!(
                input.direct_calls.get(&nested),
                Some(DirectProcDecision::Selected(_))
            ) {
                return decline(SemanticCallDecline::NestedDirectUnavailable { outer_argument });
            }
            arguments.push(SemanticCallArgument::NestedDirect {
                outer_argument,
                call: nested,
            });
        } else {
            arguments.push(SemanticCallArgument::Value { outer_argument });
        }
    }
    SemanticCallDecision::Selected(SemanticCallEvidence {
        operation: facts.operation,
        form: facts.form.clone().map(SemanticFormIdentity),
        context: input.context,
        dispatch_dependencies: facts.dispatch_dependencies,
        traits: facts.traits,
        arguments,
    })
}

fn command_has_execution_trace(module: &crate::ir::Module, command: &str) -> bool {
    module.traced_commands.contains(command)
        || module
            .traced_commands
            .contains(command.trim_start_matches("::"))
}

fn closed_program_linear_entry(function: &FunctionUnit) -> Option<(&Block, &SsaBlock)> {
    if !function.cfg.exception_edges.is_empty() {
        return None;
    }
    let block = function.cfg.blocks.get(&function.cfg.entry)?;
    let normal_completion = |terminator: &Option<crate::cfg::Terminator>| {
        matches!(
            terminator,
            None | Some(crate::cfg::Terminator::Return {
                value: None,
                expr: None,
                ..
            })
        )
    };
    let linear_shape = match &block.terminator {
        terminator if normal_completion(terminator) => function.cfg.blocks.len() == 1,
        Some(crate::cfg::Terminator::Goto { target, .. }) => {
            function.cfg.blocks.len() == 2
                && *target != function.cfg.entry
                && function.cfg.blocks.get(target).is_some_and(|exit| {
                    exit.statements.is_empty() && normal_completion(&exit.terminator)
                })
                && function
                    .ssa
                    .blocks
                    .get(target)
                    .is_some_and(|exit| exit.phis.is_empty() && exit.statements.is_empty())
        }
        _ => false,
    };
    linear_shape
        .then(|| function.ssa.blocks.get(&function.cfg.entry))
        .flatten()
        .filter(|ssa| ssa.statements.len() == block.statements.len())
        .map(|ssa| (block, ssa))
}

struct ClosedStatementInputs<'a, 'binding> {
    unit: &'a CompilationUnit,
    registry: &'a tcl_registry::CommandRegistry,
    mutations: &'a crate::command_binding::ModuleCommandMutations,
    direct_calls: &'a BTreeMap<DirectCallSiteId, DirectProcDecision>,
    semantic_calls: &'a BTreeMap<DirectCallSiteId, SemanticCallDecision>,
    function: &'a FunctionUnit,
    bindings: &'binding CommandBinding<'a>,
    direct_actuals: &'a HashSet<SsaValueIdentity>,
    set_operation: SemanticOperationId,
}

fn cover_closed_statement(
    input: &ClosedStatementInputs<'_, '_>,
    index: usize,
    statement: &Statement,
    ssa: &SsaStatement,
) -> Option<ClosedProgramStatementEvidence> {
    let statement_index = u32::try_from(index).unwrap_or(u32::MAX);
    let statement_id = CfgStatementId {
        function: input.function.name.clone(),
        block: input.function.cfg.entry,
        statement_index,
    };
    if matches!(statement, Statement::AssignConst { .. }) {
        if input.unit.ir_module.has_dynamic_trace
            || !semantic_operation_binding_is_trusted(
                input.registry,
                input.mutations,
                input.set_operation,
            )
            || semantic_operation_has_execution_trace(
                input.registry,
                &input.unit.ir_module,
                input.set_operation,
            )
            || !input
                .registry
                .command_names_for_semantic_operation(input.set_operation)
                .any(|command| {
                    input
                        .bindings
                        .is_original_builtin_at(input.function.cfg.entry, index, command)
                })
        {
            return None;
        }
        let (symbol, version) = ssa.defs.iter().find_map(|(symbol, version)| {
            let value = SsaValueIdentity {
                function: input.function.name.clone(),
                symbol: *symbol,
                version: *version,
            };
            input
                .direct_actuals
                .contains(&value)
                .then_some((*symbol, *version))
        })?;
        let constant = input.function.sccp.values.get(&(symbol, version))?.clone();
        return Some(ClosedProgramStatementEvidence::DirectActualConstant {
            statement: statement_id,
            value: SsaValueIdentity {
                function: input.function.name.clone(),
                symbol,
                version,
            },
            constant,
            operation: input.set_operation,
        });
    }
    let Statement::Call { .. } = statement else {
        return None;
    };
    let site = DirectCallSiteId {
        function: input.function.name.clone(),
        block: input.function.cfg.entry,
        statement_index,
        nested_argument: None,
    };
    let SemanticCallDecision::Selected(semantic) = input.semantic_calls.get(&site)? else {
        return None;
    };
    if semantic.operation == SemanticOperationId::StructuredLowering(LoweringHookId::Proc) {
        let procedure = input
            .direct_calls
            .values()
            .find_map(|decision| match decision {
                DirectProcDecision::Selected(evidence)
                    if evidence.callee.definition_start == statement.span().start()
                        && evidence.callee.definition_end == statement.span().end() =>
                {
                    Some(evidence.callee.clone())
                }
                _ => None,
            })?;
        return Some(ClosedProgramStatementEvidence::DirectProcedureDefinition {
            statement: statement_id,
            procedure,
        });
    }
    if !semantic.traits.contains(Traits::FRAMELESS_RUNTIME)
        || semantic.arguments.is_empty()
        || !semantic.arguments.iter().all(|argument| match argument {
            SemanticCallArgument::NestedDirect { call, .. } => matches!(
                input.direct_calls.get(call),
                Some(DirectProcDecision::Selected(_))
            ),
            SemanticCallArgument::Value { .. } => false,
        })
    {
        return None;
    }
    Some(ClosedProgramStatementEvidence::SemanticBoundary {
        call: site,
        operation: semantic.operation,
    })
}

fn prove_closed_program_coverage(
    unit: &CompilationUnit,
    registry: &tcl_registry::CommandRegistry,
    environment: CommonAotEnvironment,
    mutations: &crate::command_binding::ModuleCommandMutations,
    direct_calls: &BTreeMap<DirectCallSiteId, DirectProcDecision>,
    semantic_calls: &BTreeMap<DirectCallSiteId, SemanticCallDecision>,
) -> ClosedProgramCoverageDecision {
    let decline = |reason| ClosedProgramCoverageDecision::Declined(reason);
    if environment != CommonAotEnvironment::SealedProgram {
        return decline(ClosedProgramCoverageDecline::HostedEnvironment);
    }
    let function = &unit.top_level;
    let Some((block, ssa_block)) = closed_program_linear_entry(function) else {
        return decline(ClosedProgramCoverageDecline::NonLinearControlFlow);
    };

    let direct_actuals: HashSet<SsaValueIdentity> = direct_calls
        .values()
        .filter_map(|decision| match decision {
            DirectProcDecision::Selected(evidence) => Some(&evidence.actual_values),
            DirectProcDecision::Declined(_) => None,
        })
        .flatten()
        .filter_map(|actual| match actual {
            DirectActualValue::Ssa(value) => Some(value.clone()),
            DirectActualValue::Unproven => None,
        })
        .collect();
    let bindings = analyse_command_binding(&function.cfg, registry, &[]);
    let set_operation = SemanticOperationId::StructuredLowering(LoweringHookId::Set);
    let input = ClosedStatementInputs {
        unit,
        registry,
        mutations,
        direct_calls,
        semantic_calls,
        function,
        bindings: &bindings,
        direct_actuals: &direct_actuals,
        set_operation,
    };
    let mut statements = Vec::with_capacity(block.statements.len());
    for (index, (statement, ssa)) in block
        .statements
        .iter()
        .zip(&ssa_block.statements)
        .enumerate()
    {
        let Some(evidence) = cover_closed_statement(&input, index, statement, ssa) else {
            return decline(ClosedProgramCoverageDecline::UncoveredStatement);
        };
        statements.push(evidence);
    }
    ClosedProgramCoverageDecision::Selected(ClosedProgramCoverageEvidence {
        block: function.cfg.entry,
        statements,
    })
}

struct DirectInputs<'a> {
    unit: &'a CompilationUnit,
    registry: &'a tcl_registry::CommandRegistry,
    function: &'a FunctionUnit,
    site: &'a CallCandidate<'a>,
    binding_kind: BindingKind,
    binding_target: Option<&'a str>,
    callee_name: &'a str,
    proc_def: &'a Procedure,
    summary: Option<&'a ProcEscapeSummary>,
    mutations: &'a crate::command_binding::ModuleCommandMutations,
    context: Option<SemanticContext>,
    enabled: bool,
    frame_elision_enabled: bool,
    native_integer_enabled: bool,
}

fn direct_decision(input: &DirectInputs<'_>) -> DirectProcDecision {
    let decline = |reason| DirectProcDecision::Declined(reason);
    if !input.enabled {
        return decline(DirectProcDecline::PassDisabled);
    }
    if input.context.is_none() {
        return decline(DirectProcDecline::ContextUnavailable);
    }
    if input.mutations.has_dynamic_mutation() {
        return decline(DirectProcDecline::DynamicCommandMutation);
    }
    if !input.mutations.trusts_proc_binding(input.callee_name) {
        return decline(DirectProcDecline::ReboundOrAliased);
    }
    if input.binding_kind == BindingKind::Alias
        || input
            .binding_target
            .is_some_and(|target| target != input.callee_name)
    {
        return decline(DirectProcDecline::ReboundOrAliased);
    }
    if input.binding_kind != BindingKind::Proc {
        return decline(DirectProcDecline::BindingNotProcedure {
            kind: input.binding_kind,
        });
    }
    if input.unit.ir_module.has_dynamic_trace {
        return decline(DirectProcDecline::DynamicExecutionTrace);
    }
    let trace_name = input.callee_name.trim_start_matches("::");
    if input.unit.ir_module.traced_commands.contains(trace_name) {
        return decline(DirectProcDecline::ExecutionTrace);
    }
    if !input.function.cfg.exception_edges.is_empty()
        || input
            .unit
            .procedures
            .get(input.callee_name)
            .is_some_and(|callee| !callee.cfg.exception_edges.is_empty())
    {
        return decline(DirectProcDecline::ExceptionalControlFlow);
    }
    let Ok(formals) =
        crate::signature_scan::params::parse_param_list_strict(&input.proc_def.params_raw)
    else {
        return decline(DirectProcDecline::InvalidFormalList);
    };
    if formals.last().is_some_and(|formal| formal.name == "args") {
        return decline(DirectProcDecline::VariadicUnsupported);
    }
    if formals.iter().any(|formal| formal.default.is_some()) {
        return decline(DirectProcDecline::DefaultArgumentUnsupported);
    }
    if formals.len() != input.site.args.len() {
        return decline(DirectProcDecline::ArityMismatch {
            expected: formals.len(),
            actual: input.site.args.len(),
        });
    }
    let Some(summary) = input.summary else {
        return decline(DirectProcDecline::DynamicCallee);
    };
    let actual_facts: Vec<_> = input
        .site
        .args
        .iter()
        .map(|argument| actual_fact(input.function, input.site.ssa_statement, argument))
        .collect();
    let actual_types = actual_facts.iter().map(|fact| fact.0.clone()).collect();
    let actual_values = actual_facts.into_iter().map(|fact| fact.1).collect();
    let body = direct_body_decision(input, &formals);
    let frame_elidable =
        matches!(body, DirectProcBodyDecision::Selected(_)) && summary.safe_for_frame_elision();
    DirectProcDecision::Selected(DirectProcEvidence {
        callee: ProcedureIdentity {
            qualified_name: input.callee_name.to_owned(),
            definition_start: input.proc_def.span.start(),
            definition_end: input.proc_def.span.end(),
        },
        formals: formals.into_iter().map(|formal| formal.name).collect(),
        actual_types,
        actual_values,
        context: input.context,
        dispatch_dependencies: DispatchDependencies::BASE.union(DispatchDependencies::one(
            DispatchDependencyDomain::UnknownHandling,
        )),
        body,
        frame_escape_private: summary.safe_for_frame_elision(),
        frame_elidable,
    })
}

fn direct_body_decision(
    input: &DirectInputs<'_>,
    formals: &[tcl_syntax::formal_params::FormalParameter],
) -> DirectProcBodyDecision {
    let decline = |reason| DirectProcBodyDecision::Declined(reason);
    if !input.frame_elision_enabled {
        return decline(DirectProcBodyDecline::FrameElisionPassDisabled);
    }
    if !input.native_integer_enabled {
        return decline(DirectProcBodyDecline::NativeIntegerPassDisabled);
    }
    let [
        Statement::Return {
            expr: Some(expr), ..
        },
    ] = input.proc_def.body.statements.as_slice()
    else {
        return decline(DirectProcBodyDecline::UnsupportedBodyShape);
    };
    let formal_names: HashSet<&str> = formals.iter().map(|formal| formal.name.as_str()).collect();
    if !closed_integer_add_expression(expr, &formal_names) {
        return decline(DirectProcBodyDecline::UnsupportedBodyShape);
    }
    let operations = vec![
        SemanticOperationId::StructuredLowering(LoweringHookId::Expr),
        SemanticOperationId::StructuredLowering(LoweringHookId::Return),
    ];
    for &operation in &operations {
        if !semantic_operation_binding_is_trusted(input.registry, input.mutations, operation) {
            return decline(DirectProcBodyDecline::InternalDispatchUntrusted { operation });
        }
        if semantic_operation_has_execution_trace(input.registry, &input.unit.ir_module, operation)
        {
            return decline(DirectProcBodyDecline::InternalExecutionTrace { operation });
        }
    }
    DirectProcBodyDecision::Selected(DirectProcBodyEvidence {
        operations,
        dispatch_dependencies: DispatchDependencies::CONSERVATIVE,
    })
}

fn closed_integer_add_expression(
    expression: &tcl_syntax::expr::ast::ExprNode,
    formals: &HashSet<&str>,
) -> bool {
    match expression {
        tcl_syntax::expr::ast::ExprNode::Var { name, .. } => formals.contains(name.as_str()),
        tcl_syntax::expr::ast::ExprNode::Binary {
            op: tcl_syntax::expr::ast::BinOp::Add,
            left,
            right,
        } => {
            closed_integer_add_expression(left, formals)
                && closed_integer_add_expression(right, formals)
        }
        _ => false,
    }
}

fn semantic_operation_has_execution_trace(
    registry: &tcl_registry::CommandRegistry,
    module: &crate::ir::Module,
    operation: SemanticOperationId,
) -> bool {
    registry
        .command_names_for_semantic_operation(operation)
        .any(|command| {
            module.traced_commands.contains(command)
                || module
                    .traced_commands
                    .contains(command.trim_start_matches("::"))
        })
}

fn actual_fact(
    function: &FunctionUnit,
    statement: &crate::ssa::SsaStatement,
    argument: &str,
) -> (TypeLattice, DirectActualValue) {
    if let Some(name) = crate::value_shapes::whole_word_scalar_var_name(argument)
        && let Some(symbol) = function.ssa.var_symbol(name)
        && let Some(version) = statement.uses.get(&symbol)
    {
        let ty = function
            .types
            .get(&(symbol, *version))
            .cloned()
            .unwrap_or_else(TypeLattice::unknown);
        return (
            ty,
            DirectActualValue::Ssa(SsaValueIdentity {
                function: function.name.clone(),
                symbol,
                version: *version,
            }),
        );
    }
    // The compatibility CFG stores flattened argument text rather than the
    // executable WordExpr. It is unsound to infer a literal type here because
    // that text may have originated from a template or substitution. Exact
    // variable uses retain SSA identity above; every other word stays unknown
    // until this proof is keyed directly from executable IR.
    (TypeLattice::unknown(), DirectActualValue::Unproven)
}

fn ssa_value_keys(function: &FunctionUnit, proc_def: Option<&Procedure>) -> BTreeSet<ValueKey> {
    let mut keys = BTreeSet::new();
    for block in function.ssa.blocks.values() {
        for phi in &block.phis {
            keys.insert((phi.name, phi.version));
        }
        for statement in &block.statements {
            keys.extend(
                statement
                    .defs
                    .iter()
                    .map(|(symbol, version)| (*symbol, *version)),
            );
            keys.extend(
                statement
                    .uses
                    .iter()
                    .map(|(symbol, version)| (*symbol, *version)),
            );
        }
    }
    if let Some(proc_def) = proc_def {
        for parameter in &proc_def.params {
            if let Some(symbol) = function.ssa.var_symbol(parameter) {
                keys.insert((symbol, 0));
            }
        }
    }
    keys
}

fn collect_materialisable_slots(
    unit: &CompilationUnit,
    escape: &HashMap<String, ProcEscapeSummary>,
    propagated: &HashMap<(String, usize), Option<TypeLattice>>,
    config: SemanticOptimisationConfig,
    environment: CommonAotEnvironment,
    closed_program_coverage: &ClosedProgramCoverageDecision,
) -> BTreeMap<SsaValueIdentity, MaterialisableSlotDecision> {
    let mut slots = BTreeMap::new();
    for (qname, function) in
        std::iter::once((&unit.top_level.name, &unit.top_level)).chain(unit.procedures.iter())
    {
        let summary = escape.get(qname);
        let local_slots = if qname == "::top" && environment == CommonAotEnvironment::SealedProgram
        {
            let names: BTreeSet<String> = match closed_program_coverage {
                ClosedProgramCoverageDecision::Selected(coverage) => coverage
                    .statements
                    .iter()
                    .filter_map(|statement| match statement {
                        ClosedProgramStatementEvidence::DirectActualConstant { value, .. } => {
                            Some(function.ssa.var_name(value.symbol).to_owned())
                        }
                        ClosedProgramStatementEvidence::DirectProcedureDefinition { .. }
                        | ClosedProgramStatementEvidence::SemanticBoundary { .. } => None,
                    })
                    .collect(),
                ClosedProgramCoverageDecision::Declined(_) => BTreeSet::new(),
            };
            names
                .into_iter()
                .take(crate::var_escape::LOCALS_ARRAY_CAP)
                .enumerate()
                .map(|(slot, name)| (name, u32::try_from(slot).unwrap_or(u32::MAX)))
                .collect()
        } else {
            summary
                .map(|summary| summary.local_slots.clone())
                .unwrap_or_default()
        };
        // The lowered module's own numeral grammar (`Module::dialect`) — the
        // same source `codegen_module` reads, so a slot decision made here and
        // the code emitted for it agree on what `0755` is.
        let intervals = compute_intervals_with(
            &function.cfg,
            &function.ssa,
            &function.sccp.values,
            numbers_for_dialect(unit.ir_module.dialect.as_deref().map(|name| {
                crate::environment_ingress::resolve_environment(name).analyser_profile()
            })),
        );
        for key in ssa_value_keys(function, unit.ir_module.procedures.get(qname)) {
            let identity = SsaValueIdentity {
                function: qname.clone(),
                symbol: key.0,
                version: key.1,
            };
            let decision = materialisable_decision(&MaterialisableInputs {
                unit,
                qname,
                function,
                key,
                summary,
                local_slots: &local_slots,
                propagated,
                intervals: &intervals,
                enabled: config.is_enabled(SemanticOptimisationPassId::MaterialisableSlot),
                environment,
            });
            slots.insert(identity, decision);
        }
    }
    slots
}

struct MaterialisableInputs<'a> {
    unit: &'a CompilationUnit,
    qname: &'a str,
    function: &'a FunctionUnit,
    key: ValueKey,
    summary: Option<&'a ProcEscapeSummary>,
    local_slots: &'a BTreeMap<String, u32>,
    propagated: &'a HashMap<(String, usize), Option<TypeLattice>>,
    intervals: &'a HashMap<ValueKey, Interval>,
    enabled: bool,
    environment: CommonAotEnvironment,
}

fn materialisable_decision(input: &MaterialisableInputs<'_>) -> MaterialisableSlotDecision {
    let decline = |reason| MaterialisableSlotDecision::Declined(reason);
    if !input.enabled {
        return decline(MaterialisableSlotDecline::PassDisabled);
    }
    if input.qname == "::top" && input.environment == CommonAotEnvironment::Hosted {
        return decline(MaterialisableSlotDecline::HostedTopLevelObservable);
    }
    if !input.function.cfg.exception_edges.is_empty() {
        return decline(MaterialisableSlotDecline::ExceptionalControlFlow);
    }
    let variable = input.function.ssa.var_name(input.key.0).to_owned();
    let Some(summary) = input.summary else {
        return decline(MaterialisableSlotDecline::NoLocalSlot);
    };
    if summary.tags.get(&variable) == Some(&EscapeTag::Frame) {
        return decline(MaterialisableSlotDecline::EscapesToFrame);
    }
    let Some(&local_slot) = input.local_slots.get(&variable) else {
        return decline(MaterialisableSlotDecline::NoLocalSlot);
    };
    if input.unit.ir_module.has_dynamic_variable_trace
        || input.unit.ir_module.traced_variables.contains(&variable)
    {
        return decline(MaterialisableSlotDecline::VariableTrace);
    }
    let mut ty = input
        .function
        .types
        .get(&input.key)
        .cloned()
        .unwrap_or_else(TypeLattice::unknown);
    if input.key.1 == 0
        && let Some(proc_def) = input.unit.ir_module.procedures.get(input.qname)
        && let Some(index) = proc_def.params.iter().position(|name| name == &variable)
        && let Some(propagated) = input
            .propagated
            .get(&(input.qname.to_owned(), index))
            .and_then(Option::as_ref)
    {
        ty = type_join(&ty, propagated);
    }
    let Some(shape) = ty.single_shape().cloned() else {
        return decline(MaterialisableSlotDecline::TypeNotSingleton);
    };
    MaterialisableSlotDecision::Selected(MaterialisableSlotEvidence {
        local_slot,
        variable,
        shape,
        constant: input.function.sccp.values.get(&input.key).cloned(),
        interval: input
            .intervals
            .get(&input.key)
            .copied()
            .filter(|interval| !interval.is_bottom()),
        storage: VarStorage::MaterializableSlot,
        recipe: MaterialisationRecipe::RetainOriginalTclObject {
            sharing: SharingState::Shared,
        },
        runtime_guards: MaterialisationGuardRequirements {
            variable_trace_epoch: true,
            interpreter_policy_epoch: true,
        },
    })
}

#[cfg(test)]
mod tests {
    use tcl_dialect::model::{SpecSurface};
    use super::*;
    use tcl_registry::{IntrinsicId, TclType};

    fn plan(source: &str, config: SemanticOptimisationConfig) -> CommonAotProofPlan {
        plan_in_environment(source, config, CommonAotEnvironment::Hosted)
    }

    fn plan_in_environment(
        source: &str,
        config: SemanticOptimisationConfig,
        environment: CommonAotEnvironment,
    ) -> CommonAotProofPlan {
        let registry = tcl_registry::CommandRegistry::build_default();
        let unit = CompilationUnit::build_for_dialect(source, &registry, false, "tcl9.0");
        CommonAotProofPlan::build(
            &unit,
            &registry,
            unit.top_level.semantic_facts.context(),
            config,
            environment,
        )
    }

    fn enabled() -> SemanticOptimisationConfig {
        SemanticOptimisationConfig::new()
            .with_enabled(SemanticOptimisationPassId::DirectProc)
            .with_enabled(SemanticOptimisationPassId::MaterialisableSlot)
            .with_enabled(SemanticOptimisationPassId::FrameElision)
            .with_enabled(SemanticOptimisationPassId::NativeInteger)
            .with_enabled(SemanticOptimisationPassId::SemanticOperationSpecialisation)
    }

    const ADD: &str =
        "proc add {b c} { return [expr {$b+$c}] }\nset d 2\nset e 4\nputs [add $d $e]\n";

    #[test]
    fn a_declined_caller_poisons_propagated_actual_types() {
        // `::p` is reached by a selected top-level call passing an integer and
        // by a declined call (exceptional control flow) passing a string.  The
        // propagated join covers the selected site alone, so without poisoning
        // it narrows the parameter to `Int` — while the caller-scope constant
        // for the very same slot is the declined site's `abc`.  A materialised
        // slot must not be selected from that contradiction.
        let plan = plan(
            "proc p {x} { return [expr {$x + 1}] }\n\
             proc risky {} { try { p abc } on error {} {} }\n\
             set d 7\nputs [p $d]\n",
            enabled(),
        );
        assert!(plan.has_declined_direct_call_to("::p"));
        for (id, decision) in plan.materialisable_slots() {
            assert!(
                id.function != "::p" || matches!(decision, MaterialisableSlotDecision::Declined(_)),
                "an unselected caller must leave `::p`'s parameter unproved: {decision:?}"
            );
        }
    }

    #[test]
    fn passes_are_off_by_default() {
        let plan = plan(ADD, SemanticOptimisationConfig::default());
        assert!(plan.direct_calls().any(|(_, decision)| matches!(
            decision,
            DirectProcDecision::Declined(DirectProcDecline::PassDisabled)
        )));
        assert!(plan.materialisable_slots().all(|(_, decision)| matches!(
            decision,
            MaterialisableSlotDecision::Declined(MaterialisableSlotDecline::PassDisabled)
        )));
    }

    #[test]
    fn add_call_and_integer_formals_receive_common_proofs() {
        let plan = plan(ADD, enabled());
        let direct = plan
            .direct_calls()
            .find_map(|(_, decision)| match decision {
                DirectProcDecision::Selected(evidence)
                    if evidence.callee.qualified_name == "::add" =>
                {
                    Some(evidence)
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("nested add call selected: {plan:#?}"));
        assert_eq!(direct.formals, ["b", "c"]);
        assert!(direct.frame_escape_private);
        assert!(direct.frame_elidable);
        assert!(matches!(direct.body, DirectProcBodyDecision::Selected(_)));
        assert!(
            direct
                .actual_types
                .iter()
                .all(|ty| ty.tcl_type() == Some(TclType::Int))
        );

        let selected: Vec<_> = plan
            .materialisable_slots()
            .filter_map(|(_, decision)| match decision {
                MaterialisableSlotDecision::Selected(evidence)
                    if matches!(evidence.variable.as_str(), "b" | "c") =>
                {
                    Some(evidence)
                }
                _ => None,
            })
            .collect();
        assert_eq!(selected.len(), 2);
        assert!(selected.iter().all(|slot| slot.shape == TypeShape::Int));
        assert!(selected.iter().all(|slot| {
            slot.storage == VarStorage::MaterializableSlot
                && slot.recipe
                    == MaterialisationRecipe::RetainOriginalTclObject {
                        sharing: SharingState::Shared,
                    }
        }));

        assert_eq!(direct.actual_values.len(), 2);
        assert!(direct.actual_values.iter().all(|actual| matches!(
            actual,
            DirectActualValue::Ssa(SsaValueIdentity { function, .. }) if function == "::top"
        )));

        let outer = plan
            .semantic_calls()
            .find_map(|(site, decision)| match decision {
                SemanticCallDecision::Selected(evidence)
                    if evidence.operation
                        == SemanticOperationId::Intrinsic(IntrinsicId::ChannelWrite) =>
                {
                    Some((site, evidence))
                }
                _ => None,
            })
            .expect("outer channel-write call selected");
        assert_eq!(outer.0.nested_argument, None);
        assert!(matches!(
            outer.1.arguments.as_slice(),
            [SemanticCallArgument::NestedDirect {
                outer_argument: 0,
                call
            }] if call.nested_argument == Some(0)
        ));
    }

    #[test]
    fn semantic_operation_pass_is_independently_default_off() {
        let config = enabled().with_enabled(SemanticOptimisationPassId::DirectProc);
        let mut without_semantic = config;
        without_semantic.disable(SemanticOptimisationPassId::SemanticOperationSpecialisation);
        let plan = plan(ADD, without_semantic);
        assert!(plan.semantic_calls().all(|(_, decision)| matches!(
            decision,
            SemanticCallDecision::Declined(SemanticCallDecline::PassDisabled)
        )));
    }

    #[test]
    fn semantic_operation_rebinding_alias_trace_and_dynamic_mutation_decline() {
        for (source, expected) in [
            (
                "proc add {b c} {return [expr {$b+$c}]}\nrename puts q\nq [add 2 4]\n",
                SemanticCallDecline::RegistryUnresolved,
            ),
            (
                "proc add {b c} {return [expr {$b+$c}]}\nrename puts coreputs\ninterp alias {} puts {} coreputs\nputs [add 2 4]\n",
                SemanticCallDecline::BindingNotOriginal {
                    kind: BindingKind::Alias,
                },
            ),
            (
                "proc add {b c} {return [expr {$b+$c}]}\ntrace add execution puts enter cb\nputs [add 2 4]\n",
                SemanticCallDecline::ExecutionTrace,
            ),
            (
                "proc mutate {name} {rename $name q}\nproc add {b c} {return [expr {$b+$c}]}\nputs [add 2 4]\n",
                SemanticCallDecline::DynamicCommandMutation,
            ),
        ] {
            let plan = plan(source, enabled());
            assert!(
                plan.semantic_calls().any(|(_, decision)| matches!(
                    decision,
                    SemanticCallDecision::Declined(reason) if reason == &expected
                )),
                "{source}: {plan:#?}"
            );
        }
    }

    #[test]
    fn hosted_top_level_state_is_explicitly_observable() {
        let plan = plan(ADD, enabled());
        assert_eq!(plan.environment(), CommonAotEnvironment::Hosted);
        assert!(!plan.environment().permits_native_only());
        assert!(plan.materialisable_slots().any(|(identity, decision)| {
            identity.function == "::top"
                && matches!(
                    decision,
                    MaterialisableSlotDecision::Declined(
                        MaterialisableSlotDecline::HostedTopLevelObservable
                    )
                )
        }));
        assert!(plan.materialisable_slots().all(|(_, decision)| {
            !matches!(
                decision,
                MaterialisableSlotDecision::Selected(MaterialisableSlotEvidence {
                    storage: VarStorage::NativeOnly,
                    ..
                })
            )
        }));
        assert!(CommonAotEnvironment::SealedProgram.permits_native_only());
    }

    #[test]
    fn sealed_program_proves_exact_top_slots_proc_and_output_boundary() {
        let plan = plan_in_environment(ADD, enabled(), CommonAotEnvironment::SealedProgram);
        let ClosedProgramCoverageDecision::Selected(coverage) = plan.closed_program_coverage()
        else {
            panic!("sealed program coverage missing: {plan:#?}");
        };
        assert_eq!(coverage.statements.len(), 4);
        assert!(matches!(
            coverage.statements.as_slice(),
            [
                ClosedProgramStatementEvidence::DirectProcedureDefinition { .. },
                ClosedProgramStatementEvidence::DirectActualConstant { .. },
                ClosedProgramStatementEvidence::DirectActualConstant { .. },
                ClosedProgramStatementEvidence::SemanticBoundary { .. }
            ]
        ));
        for (variable, expected) in [("d", 2), ("e", 4)] {
            assert!(
                plan.materialisable_slots().any(|(identity, decision)| {
                    identity.function == "::top"
                        && matches!(
                            decision,
                            MaterialisableSlotDecision::Selected(MaterialisableSlotEvidence {
                                variable: selected,
                                shape: TypeShape::Int,
                                constant: Some(LatticeValue::Const(
                                    crate::analyses::ConstValue::Int(value)
                                )),
                                ..
                            }) if selected == variable && *value == expected
                        )
                }),
                "missing sealed {variable}={expected}: {plan:#?}"
            );
        }

        let (direct_site, direct) = plan
            .direct_calls()
            .find_map(|(site, decision)| match decision {
                DirectProcDecision::Selected(evidence)
                    if evidence.callee.qualified_name == "::add" =>
                {
                    Some((site, evidence))
                }
                _ => None,
            })
            .expect("sealed direct add selected");
        assert!(direct.frame_elidable);
        assert!(matches!(direct.body, DirectProcBodyDecision::Selected(_)));
        assert!(plan.semantic_calls().any(|(_, decision)| matches!(
            decision,
            SemanticCallDecision::Selected(SemanticCallEvidence {
                operation: SemanticOperationId::Intrinsic(IntrinsicId::ChannelWrite),
                arguments,
                ..
            }) if matches!(
                arguments.as_slice(),
                [SemanticCallArgument::NestedDirect { outer_argument: 0, call }]
                    if call == direct_site
            )
        )));
    }

    #[test]
    fn frame_elision_has_an_independent_default_off_gate() {
        let config =
            SemanticOptimisationConfig::new().with_enabled(SemanticOptimisationPassId::DirectProc);
        let plan = plan(ADD, config);
        assert!(plan.direct_calls().any(|(_, decision)| {
            matches!(
                decision,
                DirectProcDecision::Selected(DirectProcEvidence {
                    body: DirectProcBodyDecision::Declined(
                        DirectProcBodyDecline::FrameElisionPassDisabled
                    ),
                    frame_elidable: false,
                    ..
                })
            )
        }));
    }

    #[test]
    fn internal_operation_rebinding_and_tracing_decline_body_specialisation() {
        for (source, expected_operation) in [
            (
                "proc add {b c} {return [expr {$b+$c}]}\nrename expr saved_expr\nadd 2 4\n",
                SemanticOperationId::StructuredLowering(LoweringHookId::Expr),
            ),
            (
                "proc add {b c} {return [expr {$b+$c}]}\ntrace add execution expr enter cb\nadd 2 4\n",
                SemanticOperationId::StructuredLowering(LoweringHookId::Expr),
            ),
        ] {
            let plan = plan(source, enabled());
            assert!(
                plan.direct_calls().any(|(_, decision)| match decision {
                    DirectProcDecision::Selected(evidence) => matches!(
                        evidence.body,
                        DirectProcBodyDecision::Declined(
                            DirectProcBodyDecline::InternalDispatchUntrusted { operation }
                                | DirectProcBodyDecline::InternalExecutionTrace { operation }
                        ) if operation == expected_operation
                    ),
                    DirectProcDecision::Declined(_) => false,
                }),
                "{source}: {plan:#?}"
            );
        }
    }

    #[test]
    fn dialect_tcloo_and_variable_trace_premises_are_retained() {
        let registry = tcl_registry::CommandRegistry::build_default();
        // **Enumerated delta of the ledger C1 / §11.2 D1 re-key.** This
        // assertion used to feed `SpecSurface::ALL_TCL` — a multi-bit mask no
        // production caller could produce — and pin the `count_ones() != 1`
        // decline. The executable-IR vocabulary is now a resolved
        // environment, which names exactly one context or none, so an
        // ambiguous premise is unrepresentable rather than declined. The
        // decline it pins is therefore the surviving one: a unit built with
        // no dialect carries no context.
        let unit = CompilationUnit::build_for(ADD, &registry, false);
        let ambiguous = CommonAotProofPlan::build(
            &unit,
            &registry,
            unit.top_level.semantic_facts.context(),
            enabled(),
            CommonAotEnvironment::Hosted,
        );
        assert!(ambiguous.direct_calls().any(|(_, decision)| matches!(
            decision,
            DirectProcDecision::Declined(DirectProcDecline::ContextUnavailable)
        )));

        let oo = plan(
            "oo::class create C { method m {x} { return $x } }\n",
            enabled(),
        );
        assert!(
            oo.coverage_declines()
                .contains(&CommonAotCoverageDecline::TclOoMethods)
        );

        let traced = plan(
            "proc p {x} {return $x}\ntrace add variable x read cb\np 1\n",
            enabled(),
        );
        assert!(traced.materialisable_slots().any(|(_, decision)| matches!(
            decision,
            MaterialisableSlotDecision::Declined(MaterialisableSlotDecline::VariableTrace)
        )));
    }

    #[test]
    fn unknown_actual_type_poisons_cross_call_formal_propagation() {
        let plan = plan(
            "proc add {b c} {return [expr {$b+$c}]}\nset d 2\nset e 4\nadd $d $e\nset x [lindex $argv 0]\nadd $d $x\n",
            enabled(),
        );
        assert!(
            plan.materialisable_slots().any(|(identity, decision)| {
                identity.function == "::add"
                    && matches!(
                        decision,
                        MaterialisableSlotDecision::Declined(
                            MaterialisableSlotDecline::TypeNotSingleton
                        )
                    )
            }),
            "{plan:#?}"
        );
    }

    #[test]
    fn defaults_variadics_rebinding_aliases_and_traces_decline() {
        for (source, expected) in [
            (
                "proc p {{x 1}} {return $x}\np\n",
                DirectProcDecline::DefaultArgumentUnsupported,
            ),
            (
                "proc p {args} {return $args}\np 1\n",
                DirectProcDecline::VariadicUnsupported,
            ),
            (
                "proc p {x} {return $x}\nrename p q\nq 1\n",
                DirectProcDecline::ReboundOrAliased,
            ),
            (
                "proc p {x} {return $x}\ninterp alias {} q {} p\nq 1\n",
                DirectProcDecline::ReboundOrAliased,
            ),
            (
                "proc p {x} {return $x}\ntrace add execution p enter cb\np 1\n",
                DirectProcDecline::ExecutionTrace,
            ),
        ] {
            let plan = plan(source, enabled());
            assert!(
                plan.direct_calls().any(|(_, decision)| {
                    matches!(decision, DirectProcDecision::Declined(reason) if reason == &expected)
                }),
                "{source}: {plan:?}"
            );
        }
    }

    #[test]
    fn exceptional_edges_decline_direct_and_slot_plans() {
        let plan = plan(
            "proc p {x} {try {return [expr {$x+1}]} on error e {return $e}}\np 1\n",
            enabled(),
        );
        assert!(plan.direct_calls().any(|(_, decision)| matches!(
            decision,
            DirectProcDecision::Declined(DirectProcDecline::ExceptionalControlFlow)
        )));
        assert!(plan.materialisable_slots().any(|(_, decision)| matches!(
            decision,
            MaterialisableSlotDecision::Declined(MaterialisableSlotDecline::ExceptionalControlFlow)
        )));
    }
}
