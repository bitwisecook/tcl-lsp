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
//! shared independently selected parameter grammar.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use tcl_registry::model::semantic::SemanticContext;
use tcl_registry::{DispatchDependencies, DispatchDependencyDomain, Traits};
use tcl_registry::{SemanticOperationId, hooks::LoweringHookId};

use crate::analyses::LatticeValue;
use crate::cfg::{Block, BlockId};
use crate::command_binding::BindingKind;
use crate::compilation_unit::{CompilationUnit, FunctionUnit};
use crate::intervals::{Interval, compute_intervals_with};
use crate::ir::{CommandTokens, Procedure, Statement};
use crate::native_integer_proof::NativeIntegerDeclineReason;
use crate::registry_invocation::resolve_command_tokens;
use crate::registry_invocation::{
    InvocationMetadataContext, RegistryInvocationResolution,
    resolve_command_tokens_with_metadata_context,
};
use crate::representation_plan::{SharingState, VarStorage};
use crate::semantic_optimisation::{SemanticOptimisationConfig, SemanticOptimisationPassId};
use crate::ssa::{SsaBlock, SsaStatement, Symbol, ValueKey};
use crate::types::{TypeKind, TypeLattice, TypeShape, type_join};
use crate::var_escape::{EscapeTag, ProcEscapeSummary, analyse_var_escape_cu_with_registry};

mod declared_arguments;
pub use declared_arguments::{
    DeclaredArgumentDecision, DeclaredArgumentEvidence, DeclaredArgumentIdentity,
};

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
    /// Checked display of fixed required formal keys, in argument order.
    pub formals: Vec<String>,
    /// Genuine counted declaration/argument ordinals, separate from native slots.
    pub original_formals:
        Option<std::sync::Arc<crate::var_escape::original_slots::OriginalScalarArgumentSlots>>,
    /// Types propagated from this call's already-evaluated actual arguments.
    pub actual_types: Vec<TypeLattice>,
    /// Exact caller values corresponding to each formal, when retained by SSA.
    pub actual_values: Vec<DirectActualValue>,
    /// Explicit standalone context, when the caller supplied one.
    pub context: Option<SemanticContext>,
    /// Complete selected availability; independent Native premises remain required.
    pub metadata_context: tcl_registry::model::ResolvedContext,
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

impl DirectProcBodyDecline {
    /// Stable Explorer/API spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FrameElisionPassDisabled => "frame-elision-pass-disabled",
            Self::NativeIntegerPassDisabled => "native-integer-pass-disabled",
            Self::UnsupportedBodyShape => "unsupported-body-shape",
            Self::InternalDispatchProofUnavailable => "internal-dispatch-proof-unavailable",
            Self::InternalDispatchUntrusted { .. } => "internal-dispatch-untrusted",
            Self::InternalExecutionTrace { .. } => "internal-execution-trace",
        }
    }
}

/// Whether every registry spelling of one semantic operation retains its
/// declared binding in this module.
///
/// This target-neutral query is shared by common proof construction and the
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
    /// A genuine provider must admit the body before validating its formals.
    NativeCompilationAdmissionRequired,
    /// Actual caller or callee constants depend on math bindings which this
    /// direct execution plan cannot retain and validate.
    MathBindingPrerequisiteRequired,
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
    /// A dynamic command-binding transition can invalidate every declared
    /// procedure identity.
    DynamicCommandMutation,
    /// An execution trace with a dynamic target can observe any call.
    DynamicExecutionTrace,
    /// This procedure has an execution trace, including step traces.
    ExecutionTrace,
    /// Tcl's strict formal-list parser rejected the retained declaration.
    InvalidFormalList,
    /// Defaulted parameters are deliberately outside this first direct tier.
    DefaultArgumentUnsupported,
    /// A native rest binding is outside this first direct tier.
    VariadicUnsupported,
    /// Native caller-variable reference bindings need the ordinary frame owner.
    ReferenceArgumentUnsupported,
    /// Multiple formals share one native name rather than independent direct slots.
    SharedFormalSlotUnsupported,
    /// The already-evaluated argv does not match the fixed formal count.
    ArityMismatch {
        /// Number of fixed required formals.
        expected: usize,
        /// Number of already-evaluated actual arguments.
        actual: usize,
    },
    /// The direct tier cannot preserve an original argv expansion operation.
    ExpandedArgumentsUnsupported,
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
            Self::NativeCompilationAdmissionRequired => "native-compilation-admission-required",
            Self::MathBindingPrerequisiteRequired => "math-binding-prerequisite-required",
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
            Self::ReferenceArgumentUnsupported => "reference-argument-unsupported",
            Self::SharedFormalSlotUnsupported => "shared-formal-slot-unsupported",
            Self::ArityMismatch { .. } => "arity-mismatch",
            Self::ExpandedArgumentsUnsupported => "expanded-arguments-unsupported",
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
    /// Explicit standalone context, when the caller supplied one.
    pub context: Option<SemanticContext>,
    /// Complete availability used for registry resolution.
    pub metadata_context: tcl_registry::model::ResolvedContext,
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
    /// Independent byte/frame or explicit authored allocation selecting the slot.
    pub authority: MaterialisableSlotAuthority,
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

/// Purpose of a selected allocation. Source rewrite ordinals never describe a
/// physical local cache, and an advisory label cannot construct a native receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MaterialisableSlotAuthority {
    /// Explicit logical analysis of a symbolic authored cell.
    Authored,
    /// Fixed argument ordinal for a newly compiled original declaration.
    SourceFormal {
        /// Original declaration, counted arguments and declaration compiler purpose.
        declaration: std::sync::Arc<
            crate::var_escape::original_slots::OriginalDeclaredProcedureArgumentSlots,
        >,
        /// Exact local activation represented by SSA.
        cell: crate::var_resolve::VariableCellKey,
    },
    /// Exact source cell accounted for by the sealed-program statement proof.
    SealedProgramCell {
        /// Original namespace and root storage key, without a label conversion.
        cell: crate::var_resolve::VariableCellKey,
    },
    /// Actual retained native frame layout and its required incarnation.
    NativeFrame(Box<crate::var_escape::original_slots::OriginalNativeFrameSlot>),
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
    /// This slot's analysis world retains implicit math prerequisites which
    /// the materialisation plan cannot validate before native execution.
    MathBindingPrerequisiteRequired,
    /// The pass is disabled by default and was not explicitly enabled.
    PassDisabled,
    /// Escape analysis did not allocate a local slot for this name.
    NoLocalSlot,
    /// A typed original cell lacks its own byte/frame allocation receipt.
    OriginalSlotUnavailable,
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
            Self::MathBindingPrerequisiteRequired => "math-binding-prerequisite-required",
            Self::PassDisabled => "pass-disabled",
            Self::NoLocalSlot => "no-local-slot",
            Self::OriginalSlotUnavailable => "original-slot-unavailable",
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
    Selected(Box<MaterialisableSlotEvidence>),
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

impl CommonAotCoverageDecline {
    /// Stable Explorer/API spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TclOoMethods => "tcl-oo-methods",
            Self::SyntheticBodyUnits => "synthetic-body-units",
        }
    }
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

impl ClosedProgramCoverageDecline {
    /// Stable Explorer/API spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::HostedEnvironment => "hosted-environment",
            Self::NonLinearControlFlow => "non-linear-control-flow",
            Self::UncoveredStatement => "uncovered-statement",
        }
    }
}

/// Exact coverage evidence or a conservative typed decline.
#[derive(Debug, Clone, PartialEq)]
pub enum ClosedProgramCoverageDecision {
    /// Every top-level statement is explicitly accounted for.
    Selected(ClosedProgramCoverageEvidence),
    /// A backend must retain general top-level execution.
    Declined(ClosedProgramCoverageDecline),
}

/// A condition the sealed native integer addition requires, answered by a
/// compile option or by one common proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativePremise {
    /// The target lowers semantic plans at all.
    SemanticPlans,
    /// The package shape is one executable IR expresses.
    Packaging,
    /// The compiled program owns the interpreter lifetime and its final state.
    SealedProgram,
    /// A common pass the composition consumes is enabled.
    Pass(SemanticOptimisationPassId),
    /// The unit holds no surface the common tier deliberately excludes.
    Coverage,
    /// A binding-safe direct call to an in-unit procedure.
    DirectCall,
    /// The callee's body may run specialised.
    DirectBody,
    /// Every top-level statement is accounted for by the addition.
    ClosedProgram,
    /// The callee's frame may be omitted.
    Frame,
    /// Both actuals are materialisable integer slots.
    Actuals,
    /// A registry-proved boundary consumes the result.
    Boundary,
    /// The integer proof accepts the addition.
    NativeInteger,
    /// The proved operand values are the constants the program defines.
    Operands,
}

impl NativePremise {
    /// Stable Explorer/API spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SemanticPlans => "semantic-plans",
            Self::Packaging => "packaging",
            Self::SealedProgram => "sealed-program",
            Self::Pass(_) => "pass",
            Self::Coverage => "coverage",
            Self::DirectCall => "direct-call",
            Self::DirectBody => "direct-body",
            Self::ClosedProgram => "closed-program",
            Self::Frame => "frame",
            Self::Actuals => "actuals",
            Self::Boundary => "boundary",
            Self::NativeInteger => "native-integer",
            Self::Operands => "operands",
        }
    }
}

/// Why one premise of the sealed native integer addition was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeDeclineReason {
    /// A pass the composition consumes is not enabled.
    PassDisabled,
    /// The caller's isolated host implements only the source-evaluation ABI.
    EvalOnlyTestHost,
    /// The standalone bootstrap is not expressed by executable IR.
    StandaloneBootstrap,
    /// Hosted compilation preserves externally observable interpreter state.
    HostedEnvironment,
    /// The unit holds surfaces the common tier deliberately excludes.
    ExcludedSurfaces(Vec<CommonAotCoverageDecline>),
    /// No call in the unit resolved to one of its own procedures.
    NoDirectCall,
    /// The direct-procedure proof declined the call.
    DirectCall(DirectProcDecline),
    /// The body of the directly called procedure cannot run specialised.
    DirectBody(DirectProcBodyDecline),
    /// Exact sealed-program statement accounting was unavailable.
    ClosedProgram(ClosedProgramCoverageDecline),
    /// The accounted statements are not the procedure definition, two
    /// constants and the boundary this addition consumes.
    ClosedProgramShape,
    /// Escape analysis found the callee's frame observable.
    FrameEscapes,
    /// The frame may not be omitted: the body or the frame pass is not selected.
    FrameNotElidable,
    /// The call does not pass exactly two fixed formals two actuals.
    OperandCount,
    /// Lowering retained no exact value identity for an actual.
    ActualUnproven,
    /// The slot proof declined an actual.
    Slot(MaterialisableSlotDecline),
    /// An actual's slot is proved, but not as an integer.
    SlotNotInteger,
    /// The slot proof holds no decision for an actual.
    NoSlotDecision,
    /// No selected registry boundary consumes the call's result.
    NoSelectedBoundary,
    /// The callee has no function unit to analyse.
    ProofFunctionUnavailable,
    /// The complexity guard disabled the integer analysis.
    ProofComplexityGuarded,
    /// The actual source metadata needed by the integer proof is unavailable.
    ProofSourceMetadataUnavailable,
    /// The integer proof lacks the selected math command binding premise.
    ProofMathBindingPrerequisiteRequired,
    /// The callee holds no addition of two variables.
    NoAddCandidate,
    /// The callee holds more than one candidate addition.
    AmbiguousAdd,
    /// The integer proof declined the sole addition.
    Integer(NativeIntegerDeclineReason),
    /// The proved addition is not a non-overflowing return of the callee.
    AddShape,
    /// The proof rests on callers beyond the selected call.
    CallerSetDiffers,
    /// An operand's proved range is not a single value.
    OperandNotExact,
    /// The proved operands differ from the constants the program defines.
    OperandsDiffer,
}

impl NativeDeclineReason {
    /// Stable Explorer/API spelling.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::PassDisabled => "pass-disabled",
            Self::EvalOnlyTestHost => "eval-only-test-host",
            Self::StandaloneBootstrap => "standalone-bootstrap",
            Self::HostedEnvironment => "hosted-environment",
            Self::ExcludedSurfaces(_) => "excluded-surface",
            Self::NoDirectCall => "no-direct-call",
            Self::DirectCall(decline) => decline.as_str(),
            Self::DirectBody(decline) => decline.as_str(),
            Self::ClosedProgram(decline) => decline.as_str(),
            Self::ClosedProgramShape => "closed-program-shape",
            Self::FrameEscapes => "frame-escapes",
            Self::FrameNotElidable => "frame-not-elidable",
            Self::OperandCount => "operand-count",
            Self::ActualUnproven => "actual-unproven",
            Self::Slot(decline) => decline.as_str(),
            Self::SlotNotInteger => "slot-not-integer",
            Self::NoSlotDecision => "no-slot-decision",
            Self::NoSelectedBoundary => "no-selected-boundary",
            Self::ProofFunctionUnavailable => "function-unavailable",
            Self::ProofComplexityGuarded => "complexity-guarded",
            Self::ProofSourceMetadataUnavailable => "proof-source-metadata-unavailable",
            Self::ProofMathBindingPrerequisiteRequired => {
                "proof-math-binding-prerequisite-required"
            }
            Self::NoAddCandidate => "no-add-candidate",
            Self::AmbiguousAdd => "ambiguous-add",
            Self::Integer(reason) => reason.as_str(),
            Self::AddShape => "add-shape",
            Self::CallerSetDiffers => "caller-set-differs",
            Self::OperandNotExact => "operand-not-exact",
            Self::OperandsDiffer => "operands-differ",
        }
    }
}

/// One premise the sealed native integer addition rejected, and why.
///
/// A compile that does not select the addition records one of these for every
/// premise it evaluated and found wanting, in evaluation order, so the record
/// names each obstacle rather than the first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeDecline {
    /// The condition that failed.
    pub premise: NativePremise,
    /// The typed reason it failed.
    pub reason: NativeDeclineReason,
    /// The direct call sites it failed at; empty for a premise about the
    /// unit or its options.
    pub sites: Vec<DirectCallSiteId>,
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
    declared_arguments: BTreeMap<DeclaredArgumentIdentity, DeclaredArgumentDecision>,
    coverage_declines: Vec<CommonAotCoverageDecline>,
}

#[derive(Clone, Copy)]
enum AotMetadataSelection {
    Standalone(Option<SemanticContext>),
    Retained,
}

impl AotMetadataSelection {
    fn for_function<'a>(
        self,
        function: &'a FunctionUnit,
        registry: &tcl_registry::CommandRegistry,
        module: &crate::ir::Module,
    ) -> Option<InvocationMetadataContext<'a>> {
        match self {
            Self::Standalone(context) => context.map(InvocationMetadataContext::from),
            Self::Retained => function.invocation_metadata_context_for_module(registry, module),
        }
    }

    const fn standalone_context(self) -> Option<SemanticContext> {
        match self {
            Self::Standalone(context) => context,
            Self::Retained => None,
        }
    }
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
        Self::build_with_selection(
            unit,
            registry,
            AotMetadataSelection::Standalone(context),
            config,
            environment,
        )
    }

    /// Build from each function's actual retained availability and grammar.
    /// Missing, foreign or stale owners cannot supply metadata. Binding,
    /// dispatch, frame and Native execution obligations remain independent.
    #[must_use]
    pub fn build_with_retained_metadata(
        unit: &CompilationUnit,
        registry: &tcl_registry::CommandRegistry,
        config: SemanticOptimisationConfig,
        environment: CommonAotEnvironment,
    ) -> Self {
        Self::build_with_selection(
            unit,
            registry,
            AotMetadataSelection::Retained,
            config,
            environment,
        )
    }

    fn build_with_selection(
        unit: &CompilationUnit,
        registry: &tcl_registry::CommandRegistry,
        selection: AotMetadataSelection,
        config: SemanticOptimisationConfig,
        environment: CommonAotEnvironment,
    ) -> Self {
        // The registry this unit was lowered under, not `tcl8.6`: the three
        // calls below already take it, and an escape analysis answering from
        // a different dialect can under-report escaping variables (#2167).
        let escape = analyse_var_escape_cu_with_registry(unit, true, registry);
        let mutations = &unit.command_mutations;
        let direct = collect_direct_calls(
            unit,
            registry,
            selection,
            config,
            &escape,
            mutations,
            &unit.caller_scope.proc_binding_trust,
        );
        let direct_calls = direct.decisions;
        let declined_direct_callees = direct.declined_callees;
        let propagated = direct.propagated;

        let semantic_calls =
            collect_semantic_calls(unit, registry, selection, config, mutations, &direct_calls);

        let closed_program_coverage = prove_closed_program_coverage(
            unit,
            registry,
            environment,
            mutations,
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

        let declared_arguments =
            declared_arguments::collect(unit, registry, selection, &escape, &propagated, config);

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
            declared_arguments,
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

    /// Proposed declared argument storage, independent of physical SSA slots.
    pub fn declared_arguments(
        &self,
    ) -> impl Iterator<Item = (&DeclaredArgumentIdentity, &DeclaredArgumentDecision)> {
        self.declared_arguments.iter()
    }

    /// Join an exact incoming read to its canonical original declaration.
    /// A slot label alone or a different source occurrence cannot select this
    /// proposed argument, and no physical SSA symbol is created.
    #[must_use]
    pub fn declared_argument_for_incoming_read(
        &self,
        procedure: &ProcedureIdentity,
        read: &crate::ssa::SsaIncomingSlotRead,
    ) -> Option<(&DeclaredArgumentIdentity, &DeclaredArgumentEvidence)> {
        self.declared_arguments
            .iter()
            .find_map(|(identity, decision)| {
                if &identity.procedure != procedure {
                    return None;
                }
                let DeclaredArgumentDecision::Selected(evidence) = decision else {
                    return None;
                };
                (evidence.reads().contains(read)
                    && evidence
                        .declaration()
                        .arguments()
                        .ordinal(read.slot.as_bytes())
                        == Some(identity.ordinal))
                .then_some((identity, evidence.as_ref()))
            })
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
    tokens: Option<std::borrow::Cow<'a, CommandTokens>>,
}

fn function_units(unit: &CompilationUnit) -> impl Iterator<Item = (&str, &FunctionUnit)> {
    std::iter::once((unit.top_level.name.as_str(), &unit.top_level)).chain(
        unit.procedures
            .iter()
            .map(|(name, unit)| (name.as_str(), unit)),
    )
}

fn call_sites<'a>(
    caller: &str,
    function: &'a FunctionUnit,
    lexer_config: tcl_lexer::LexerConfig,
) -> Vec<CallCandidate<'a>> {
    let mut out = Vec::new();
    for (block, cfg_block) in &function.cfg.blocks {
        let Some(ssa_block) = function.ssa.blocks.get(block) else {
            continue;
        };
        for (index, statement) in cfg_block.statements.iter().enumerate() {
            if ssa_block.statements.get(index).is_none() {
                continue;
            }
            let original = crate::ir::CommandBindingSites::unanimous_statement_source_tokens(
                &function.cfg.command_binding_sites,
                statement,
            );
            let (command, args) = match statement {
                Statement::Call { command, args, .. } => (command.clone(), args.clone()),
                _ => {
                    let Some(tokens) = original.filter(|tokens| {
                        tokens.synthetic.is_none() && tokens.words_align_with_argv_text()
                    }) else {
                        continue;
                    };
                    let Some((head, arguments)) = tokens.argv_texts.split_first() else {
                        continue;
                    };
                    (head.clone(), arguments.to_vec())
                }
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
                command,
                args: args.clone(),
                tokens: original.map(std::borrow::Cow::Borrowed),
            });
            for argument_index in 0..args.len() {
                let Some(tokens) = nested_call_tokens(original, argument_index, lexer_config)
                else {
                    continue;
                };
                let Some((nested, nested_args)) = tokens.argv_texts.split_first() else {
                    continue;
                };
                out.push(CallCandidate {
                    id: DirectCallSiteId {
                        function: caller.to_owned(),
                        block: *block,
                        statement_index,
                        nested_argument: Some(u32::try_from(argument_index).unwrap_or(u32::MAX)),
                    },
                    block: *block,
                    statement_index,
                    command: nested.clone(),
                    args: nested_args.to_vec(),
                    tokens: Some(std::borrow::Cow::Owned(tokens)),
                });
            }
        }
    }
    out
}

fn nested_call_tokens(
    parent: Option<&CommandTokens>,
    argument: usize,
    config: tcl_lexer::LexerConfig,
) -> Option<CommandTokens> {
    let parent = parent?;
    let word = parent.words().get(argument.checked_add(1)?)?;
    let mut commands =
        crate::value_shapes::command_substitution_tokens(word, Some(parent), config)?;
    (commands.len() == 1).then(|| commands.remove(0))
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
    selection: AotMetadataSelection,
    config: SemanticOptimisationConfig,
    escape: &HashMap<String, ProcEscapeSummary>,
    mutations: &crate::command_binding::ModuleCommandMutations,
    proc_binding_trust: &crate::command_binding::ProcBindingTrustProjection,
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
        for site in call_sites(caller_name, function, function.source_lexer_config()) {
            let binding = site
                .tokens
                .as_deref()
                .and_then(|tokens| tokens.source_binding.as_ref());
            let terminal = binding
                .and_then(crate::command_binding::SourceInvocationBinding::proved_execution_target);
            let target = binding.and_then(|binding| binding.direct_procedure_target(&site.command));
            let resolved = terminal
                .and_then(|target| {
                    unit.ir_module
                        .procedures
                        .iter()
                        .find_map(|(name, procedure)| {
                            target
                                .matches_authored_implementation_image(
                                    &unit.ir_module.source,
                                    procedure.span.start(),
                                )
                                .then(|| name.clone())
                        })
                })
                .or_else(|| {
                    crate::interprocedural::resolve_internal_call(
                        &site.command,
                        caller_name,
                        &known,
                    )
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
                binding_kind: terminal.map_or(BindingKind::Unknown, |target| target.kind),
                binding_target: terminal.map(|target| target.command.as_str()),
                source_target: target,
                callee_name: &callee_name,
                proc_def,
                summary: escape.get(&callee_name),
                mutations,
                proc_binding_trust,
                context: selection.standalone_context(),
                metadata: selection.for_function(function, registry, &unit.ir_module),
                callee_metadata: unit
                    .function(&callee_name)
                    .and_then(|callee| selection.for_function(callee, registry, &unit.ir_module)),
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
    selection: AotMetadataSelection,
    config: SemanticOptimisationConfig,
    mutations: &crate::command_binding::ModuleCommandMutations,
    direct_calls: &BTreeMap<DirectCallSiteId, DirectProcDecision>,
) -> BTreeMap<DirectCallSiteId, SemanticCallDecision> {
    let mut decisions = BTreeMap::new();
    for (caller, function) in function_units(unit) {
        for site in call_sites(caller, function, function.source_lexer_config()) {
            if site.id.nested_argument.is_some() {
                continue;
            }
            let decision = semantic_call_decision(&SemanticCallInputs {
                unit,
                registry,
                function,
                context: selection.standalone_context(),
                metadata: selection.for_function(function, registry, &unit.ir_module),
                site: &site,
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
    function: &'a FunctionUnit,
    context: Option<SemanticContext>,
    metadata: Option<InvocationMetadataContext<'a>>,
    site: &'a CallCandidate<'a>,
    mutations: &'a crate::command_binding::ModuleCommandMutations,
    direct_calls: &'a BTreeMap<DirectCallSiteId, DirectProcDecision>,
    enabled: bool,
}

fn semantic_call_decision(input: &SemanticCallInputs<'_>) -> SemanticCallDecision {
    let decline = |reason| SemanticCallDecision::Declined(reason);
    if !input.enabled {
        return decline(SemanticCallDecline::PassDisabled);
    }
    if input.metadata.is_none() {
        return decline(SemanticCallDecline::ContextUnavailable);
    }
    if input.mutations.has_dynamic_mutation() {
        return decline(SemanticCallDecline::DynamicCommandMutation);
    }
    let Some(tokens) = input.site.tokens.as_deref() else {
        return decline(SemanticCallDecline::TokensUnavailable);
    };
    let Ok(RegistryInvocationResolution::Resolved(facts)) =
        resolve_command_tokens_with_metadata_context(input.registry, input.metadata, tokens)
    else {
        return decline(SemanticCallDecline::RegistryUnresolved);
    };
    if facts.operation == SemanticOperationId::Invoke {
        return decline(SemanticCallDecline::GenericInvocation);
    }
    let binding = tokens.source_binding.as_ref();
    if binding
        .and_then(|binding| binding.direct_registry_target(&input.site.command))
        .is_none()
    {
        let kind = binding
            .and_then(crate::command_binding::SourceInvocationBinding::called_slot_kind)
            .unwrap_or(BindingKind::Unknown);
        return decline(SemanticCallDecline::BindingNotOriginal { kind });
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
        if crate::value_shapes::parse_command_substitution_with_config(
            argument,
            input.function.source_lexer_config(),
        )
        .is_some()
        {
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
        metadata_context: input
            .metadata
            .expect("selected metadata prerequisite")
            .context()
            .clone(),
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

struct ClosedStatementInputs<'a> {
    unit: &'a CompilationUnit,
    registry: &'a tcl_registry::CommandRegistry,
    mutations: &'a crate::command_binding::ModuleCommandMutations,
    direct_calls: &'a BTreeMap<DirectCallSiteId, DirectProcDecision>,
    semantic_calls: &'a BTreeMap<DirectCallSiteId, SemanticCallDecision>,
    function: &'a FunctionUnit,
    direct_actuals: &'a HashSet<SsaValueIdentity>,
    set_operation: SemanticOperationId,
}

fn closed_constant_store(
    input: &ClosedStatementInputs<'_>,
    index: usize,
    statement: &Statement,
) -> bool {
    if matches!(statement, Statement::AssignConst { .. }) {
        return true;
    }
    let Some(tokens) = crate::ir::CommandBindingSites::unanimous_statement_source_tokens(
        &input.function.cfg.command_binding_sites,
        statement,
    ) else {
        return false;
    };
    let site = DirectCallSiteId {
        function: input.function.name.clone(),
        block: input.function.cfg.entry,
        statement_index: u32::try_from(index).unwrap_or(u32::MAX),
        nested_argument: None,
    };
    if !matches!(
        input.semantic_calls.get(&site),
        Some(SemanticCallDecision::Selected(semantic)) if semantic.operation == input.set_operation
    ) {
        return false;
    }
    let Some(binding) = tokens.source_binding.as_ref() else {
        return false;
    };
    crate::registry_invocation::normal_transfer_invocation_with_metadata_context(
        input.registry,
        input
            .function
            .invocation_metadata_context_for_module(input.registry, &input.unit.ir_module),
        tokens,
    )
    .and_then(|normal| normal.stored_value_literal(&binding.variable_context, input.registry))
    .is_some()
}

fn closed_native_store_binding(input: &ClosedStatementInputs<'_>, statement: &Statement) -> bool {
    let Statement::AssignConst { value, .. } = statement else {
        return false;
    };
    let Some(tokens) = crate::ir::CommandBindingSites::unanimous_statement_source_tokens(
        &input.function.cfg.command_binding_sites,
        statement,
    ) else {
        return false;
    };
    let Some(binding) = tokens.source_binding.as_ref() else {
        return false;
    };
    binding
        .evaluated_command_word()
        .is_some_and(|head| binding.direct_registry_target(head).is_some())
        && crate::registry_invocation::normal_transfer_invocation_with_metadata_context(
            input.registry,
            input
                .function
                .invocation_metadata_context_for_module(input.registry, &input.unit.ir_module),
            tokens,
        )
        .and_then(|normal| normal.stored_value_literal(&binding.variable_context, input.registry))
        .as_deref()
            == Some(value.as_str())
}

fn cover_closed_constant_store(
    input: &ClosedStatementInputs<'_>,
    statement: &Statement,
    ssa: &SsaStatement,
    statement_id: CfgStatementId,
) -> Option<ClosedProgramStatementEvidence> {
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
        || (matches!(statement, Statement::AssignConst { .. })
            && !closed_native_store_binding(input, statement))
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
    Some(ClosedProgramStatementEvidence::DirectActualConstant {
        statement: statement_id,
        value: SsaValueIdentity {
            function: input.function.name.clone(),
            symbol,
            version,
        },
        constant,
        operation: input.set_operation,
    })
}

fn cover_closed_statement(
    input: &ClosedStatementInputs<'_>,
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
    if closed_constant_store(input, index, statement) {
        return cover_closed_constant_store(input, statement, ssa, statement_id);
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
    let set_operation = SemanticOperationId::StructuredLowering(LoweringHookId::Set);
    let input = ClosedStatementInputs {
        unit,
        registry,
        mutations,
        direct_calls,
        semantic_calls,
        function,
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
        if !statement.is_executable_invocation() {
            continue;
        }
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
    source_target: Option<&'a crate::command_binding::SourceCommandTarget>,
    callee_name: &'a str,
    proc_def: &'a Procedure,
    summary: Option<&'a ProcEscapeSummary>,
    mutations: &'a crate::command_binding::ModuleCommandMutations,
    proc_binding_trust: &'a crate::command_binding::ProcBindingTrustProjection,
    context: Option<SemanticContext>,
    metadata: Option<InvocationMetadataContext<'a>>,
    callee_metadata: Option<InvocationMetadataContext<'a>>,
    enabled: bool,
    frame_elision_enabled: bool,
    native_integer_enabled: bool,
}

fn direct_prerequisite_decline(input: &DirectInputs<'_>) -> Option<DirectProcDecline> {
    if crate::native_compilation_admission::script_requires_admission(&input.proc_def.body) {
        return Some(DirectProcDecline::NativeCompilationAdmissionRequired);
    }
    (input.function.requires_native_math_binding_validation()
        || input
            .unit
            .function(input.callee_name)
            .is_some_and(FunctionUnit::requires_native_math_binding_validation))
    .then_some(DirectProcDecline::MathBindingPrerequisiteRequired)
}

fn direct_decision(input: &DirectInputs<'_>) -> DirectProcDecision {
    let decline = |reason| DirectProcDecision::Declined(reason);
    if !input.enabled {
        return decline(DirectProcDecline::PassDisabled);
    }
    if input.metadata.is_none() || input.callee_metadata.is_none() {
        return decline(DirectProcDecline::ContextUnavailable);
    }
    if input.proc_binding_trust.has_dynamic_binding_transition() {
        return decline(DirectProcDecline::DynamicCommandMutation);
    }
    if !input
        .proc_binding_trust
        .trusts_proc_binding(input.callee_name)
    {
        return decline(DirectProcDecline::ReboundOrAliased);
    }
    if input.unit.ir_module.has_dynamic_trace {
        return decline(DirectProcDecline::DynamicExecutionTrace);
    }
    let trace_name = input.callee_name.trim_start_matches("::");
    if input.unit.ir_module.traced_commands.contains(trace_name) {
        return decline(DirectProcDecline::ExecutionTrace);
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
    if !input.source_target.is_some_and(|target| {
        target.prepended.is_empty()
            && target.matches_authored_implementation_image(
                &input.unit.ir_module.source,
                input.proc_def.span.start(),
            )
    }) {
        return decline(DirectProcDecline::ReboundOrAliased);
    }
    if let Some(reason) = direct_prerequisite_decline(input) {
        return decline(reason);
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
    if input.site.tokens.as_deref().is_some_and(|tokens| {
        tokens
            .words()
            .iter()
            .any(|word| matches!(word, crate::ir::WordExpr::Expand { .. }))
    }) {
        return decline(DirectProcDecline::ExpandedArgumentsUnsupported);
    }
    let (formals, original_formals) = match selected_direct_formals(input) {
        Ok(selected) => selected,
        Err(reason) => return decline(reason),
    };
    let Some(summary) = input.summary else {
        return decline(DirectProcDecline::DynamicCallee);
    };
    select_direct_evidence(input, &formals, original_formals, summary)
}

fn selected_direct_formals(
    input: &DirectInputs<'_>,
) -> Result<DirectFormalSelection, DirectProcDecline> {
    let Some(grammar) = input.unit.ir_module.parameter_grammar() else {
        return Err(DirectProcDecline::ContextUnavailable);
    };
    let original_formals = crate::var_escape::original_slots::original_procedure_argument_slots(
        &input.unit.ir_module,
        input.proc_def,
    );
    let formals = if let Some(original) = &original_formals {
        if original.names().len() != input.site.args.len() {
            return Err(DirectProcDecline::ArityMismatch {
                expected: original.names().len(),
                actual: input.site.args.len(),
            });
        }
        let Some(formals) = original
            .names()
            .iter()
            .map(|name| {
                Some(tcl_syntax::formal_params::FormalParameter {
                    name: name.try_utf8().ok()?.to_owned(),
                    default: None,
                })
            })
            .collect::<Option<Vec<_>>>()
        else {
            return Err(DirectProcDecline::InvalidFormalList);
        };
        formals
    } else if input.unit.ir_module.source.is_empty()
        && input.unit.ir_module.source_entry.native_entry.is_none()
    {
        // Explicit synthetic/logical IR keeps its authored parameter grammar.
        direct_formal_bindings(&input.proc_def.params_raw, input.site.args.len(), grammar)?
    } else {
        // Preserve the established typed shape reasons without using the
        // presentation parser as an original-name issuer.
        return Err(direct_formal_bindings(
            &input.proc_def.params_raw,
            input.site.args.len(),
            grammar,
        )
        .err()
        .unwrap_or(DirectProcDecline::InvalidFormalList));
    };
    Ok((formals, original_formals))
}

type DirectFormalSelection = (
    Vec<tcl_syntax::formal_params::FormalParameter>,
    Option<crate::var_escape::original_slots::OriginalScalarArgumentSlots>,
);

fn select_direct_evidence(
    input: &DirectInputs<'_>,
    formals: &[tcl_syntax::formal_params::FormalParameter],
    original_formals: Option<crate::var_escape::original_slots::OriginalScalarArgumentSlots>,
    summary: &ProcEscapeSummary,
) -> DirectProcDecision {
    let actual_facts: Vec<_> = input
        .site
        .args
        .iter()
        .enumerate()
        .map(|(index, _)| actual_fact(input.function, input.site, index, input.registry))
        .collect();
    let actual_types = actual_facts.iter().map(|fact| fact.0.clone()).collect();
    let actual_values = actual_facts.into_iter().map(|fact| fact.1).collect();
    let body = direct_body_decision(input, formals);
    let frame_elidable =
        matches!(body, DirectProcBodyDecision::Selected(_)) && summary.safe_for_frame_elision();
    DirectProcDecision::Selected(DirectProcEvidence {
        callee: ProcedureIdentity {
            qualified_name: input.callee_name.to_owned(),
            definition_start: input.proc_def.span.start(),
            definition_end: input.proc_def.span.end(),
        },
        formals: formals.iter().map(|formal| formal.name.clone()).collect(),
        original_formals: original_formals.map(std::sync::Arc::new),
        actual_types,
        actual_values,
        context: input.context,
        metadata_context: input
            .metadata
            .expect("selected metadata prerequisite")
            .context()
            .clone(),
        dispatch_dependencies: DispatchDependencies::BASE.union(DispatchDependencies::one(
            DispatchDependencyDomain::UnknownHandling,
        )),
        body,
        frame_escape_private: summary.safe_for_frame_elision(),
        frame_elidable,
    })
}

fn direct_formal_bindings(
    source: &str,
    actual: usize,
    grammar: tcl_dialect::ParameterGrammar,
) -> Result<Vec<tcl_syntax::formal_params::FormalParameter>, DirectProcDecline> {
    use tcl_syntax::formal_params::{
        FormalArgumentBinding, bind_formal_arguments, parse_formal_parameters_in,
    };
    let formals = parse_formal_parameters_in(source, grammar)
        .map_err(|_| DirectProcDecline::InvalidFormalList)?;
    // Inspect the shared activation plan with a sufficient argument count,
    // before selecting the actual call. Native rest/reference/default slots
    // cannot be replaced by positional direct-call slots.
    let shape = bind_formal_arguments(&formals, formals.len(), grammar)
        .map_err(|_| DirectProcDecline::InvalidFormalList)?;
    for binding in &shape {
        match binding {
            FormalArgumentBinding::Rest { .. } => {
                return Err(DirectProcDecline::VariadicUnsupported);
            }
            FormalArgumentBinding::CallerLink { .. } => {
                return Err(DirectProcDecline::ReferenceArgumentUnsupported);
            }
            FormalArgumentBinding::Default { .. } | FormalArgumentBinding::Value { .. } => {}
        }
    }
    let mut slots = std::collections::HashSet::new();
    if formals.iter().any(|formal| !slots.insert(&formal.name)) {
        return Err(DirectProcDecline::SharedFormalSlotUnsupported);
    }
    if formals.iter().any(|formal| formal.default.is_some()) {
        return Err(DirectProcDecline::DefaultArgumentUnsupported);
    }
    let bindings = bind_formal_arguments(&formals, actual, grammar).map_err(|_| {
        DirectProcDecline::ArityMismatch {
            expected: formals.len(),
            actual,
        }
    })?;
    if !bindings.iter().all(|binding| {
        matches!(binding,
        FormalArgumentBinding::Value { parameter, argument } if parameter == argument)
    }) {
        return Err(DirectProcDecline::ContextUnavailable);
    }
    Ok(formals)
}

/// Original outer return and sole nested expression from one supplied body.
/// This selects metadata only; binding, frame, compiler and result-equivalence
/// premises belong to the caller's independent admission checks.
pub(crate) fn original_direct_expression_body_operations(
    registry: &tcl_registry::CommandRegistry,
    metadata: Option<InvocationMetadataContext<'_>>,
    procedure: &Procedure,
    config: tcl_lexer::LexerConfig,
) -> Option<[SemanticOperationId; 2]> {
    let metadata = metadata?;
    let [statement @ Statement::Return { expr: Some(_), .. }] =
        procedure.body.statements.as_slice()
    else {
        return None;
    };
    let original = procedure
        .body
        .retained_source_tokens_for_statement(statement)?;
    let operation = |tokens: &CommandTokens, expected| {
        if tokens.words().len() != 2 {
            return None;
        }
        let selected =
            crate::registry_invocation::logical_structured_invocation_with_metadata_context(
                registry, metadata, tokens, None,
            )?;
        (selected.lowering_hook() == Some(expected))
            .then_some(SemanticOperationId::StructuredLowering(expected))
    };
    let outer = operation(original, LoweringHookId::Return)?;
    let children = crate::word_subst::checked_lifted_calls(original, config)?;
    let [child] = children.as_slice() else {
        return None;
    };
    let nested = operation(child.tokens.as_ref()?, LoweringHookId::Expr)?;
    Some([nested, outer])
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
    let operations = [
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
    let Some(callee) = input.unit.function(input.callee_name) else {
        return decline(DirectProcBodyDecline::UnsupportedBodyShape);
    };
    let Some(operations) = original_direct_expression_body_operations(
        input.registry,
        input.callee_metadata,
        input.proc_def,
        callee.source_lexer_config(),
    ) else {
        return decline(DirectProcBodyDecline::UnsupportedBodyShape);
    };
    let [
        Statement::Return {
            expr: Some(expr), ..
        },
    ] = input.proc_def.body.statements.as_slice()
    else {
        return decline(DirectProcBodyDecline::UnsupportedBodyShape);
    };
    let formal_names: HashSet<&str> = formals.iter().map(|formal| formal.name.as_str()).collect();
    if !closed_integer_add_expression(
        expr,
        &formal_names,
        input.unit.ir_module.native_lexer_config(),
    ) {
        return decline(DirectProcBodyDecline::UnsupportedBodyShape);
    }
    DirectProcBodyDecision::Selected(DirectProcBodyEvidence {
        operations: operations.to_vec(),
        dispatch_dependencies: DispatchDependencies::CONSERVATIVE,
    })
}

fn closed_integer_add_expression(
    expression: &tcl_syntax::expr::ast::ExprNode,
    formals: &HashSet<&str>,
    config: tcl_lexer::LexerConfig,
) -> bool {
    match expression {
        tcl_syntax::expr::ast::ExprNode::Var { text, .. } => matches!(
            crate::native_lowering::cells::variable_reference_place(text, config),
            Ok(crate::native_lowering::cells::CellPlace::Named { name }) if formals.contains(name.as_str())
        ),
        tcl_syntax::expr::ast::ExprNode::Binary {
            op: tcl_syntax::expr::ast::BinOp::Add,
            left,
            right,
        } => {
            closed_integer_add_expression(left, formals, config)
                && closed_integer_add_expression(right, formals, config)
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
    site: &CallCandidate<'_>,
    argument: usize,
    registry: &tcl_registry::CommandRegistry,
) -> (TypeLattice, DirectActualValue) {
    let read = site
        .tokens
        .as_deref()
        .and_then(|tokens| tokens.words().get(argument.checked_add(1)?))
        .and_then(|word| {
            let view = crate::ssa::SsaSourceView::at_statement(
                &function.ssa,
                site.block,
                site.statement_index as usize,
            );
            if site.id.nested_argument.is_some() {
                let invocation = site
                    .tokens
                    .as_deref()?
                    .source_binding
                    .as_ref()?
                    .invocation_site()?;
                let (symbol, version) =
                    view.captured_argument_value_definition(word, invocation, registry)?;
                Some(crate::ssa::SsaReadReference {
                    symbol,
                    version: Some(version),
                })
            } else {
                view.read_word(word)
            }
        });
    if let Some(crate::ssa::SsaReadReference {
        symbol,
        version: Some(version),
    }) = read
    {
        let ty = function
            .types
            .get(&(symbol, version))
            .cloned()
            .unwrap_or_else(TypeLattice::unknown);
        return (
            ty,
            DirectActualValue::Ssa(SsaValueIdentity {
                function: function.name.clone(),
                symbol,
                version,
            }),
        );
    }
    // Neither flattened bytes nor a statement-wide use map can identify the
    // object read before later arguments run. Missing original read evidence
    // leaves the contents fact and materialisation prerequisite unproved.
    (TypeLattice::unknown(), DirectActualValue::Unproven)
}

fn direct_call_tokens<'a>(
    unit: &'a CompilationUnit,
    id: &DirectCallSiteId,
) -> Option<std::borrow::Cow<'a, CommandTokens>> {
    let function = unit.function(&id.function)?;
    let statement = function
        .ssa
        .blocks
        .get(&id.block)?
        .statements
        .get(id.statement_index as usize)?;
    let original = crate::ir::CommandBindingSites::unanimous_statement_source_tokens(
        &function.cfg.command_binding_sites,
        &statement.statement,
    )?;
    if let Some(outer) = id.nested_argument {
        Some(std::borrow::Cow::Owned(nested_call_tokens(
            Some(original),
            outer as usize,
            function.source_lexer_config(),
        )?))
    } else {
        Some(std::borrow::Cow::Borrowed(original))
    }
}

/// Contents dependency of one original direct-call value operand.
/// This proves no object representation or permission to erase its coercions.
pub(crate) fn direct_call_argument_read(
    unit: &CompilationUnit,
    id: &DirectCallSiteId,
    argument: usize,
) -> Option<crate::ssa::SsaReadReference> {
    let function = unit.function(&id.function)?;
    let tokens = direct_call_tokens(unit, id)?;
    let word = tokens.words().get(argument.checked_add(1)?)?;
    let view = crate::ssa::SsaSourceView::at_statement(
        &function.ssa,
        id.block,
        id.statement_index as usize,
    );
    if id.nested_argument.is_some() {
        let invocation = tokens.source_binding.as_ref()?.invocation_site()?;
        let (symbol, version) = view.captured_argument_value_definition(
            word,
            invocation,
            unit.ir_module.resolved_registry(),
        )?;
        Some(crate::ssa::SsaReadReference {
            symbol,
            version: Some(version),
        })
    } else {
        view.read_word(word)
    }
}

/// Exact captured contents of a selected, unexpanded direct-call operand.
/// The native tier must independently preserve the original operand evaluation.
pub(crate) fn direct_call_argument_value(
    unit: &CompilationUnit,
    id: &DirectCallSiteId,
    argument: usize,
) -> Option<String> {
    let tokens = direct_call_tokens(unit, id)?;
    tokens
        .source_binding
        .as_ref()?
        .evaluated_written_argument_value(argument)
        .map(str::to_owned)
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
        let sealed_cells = if qname == "::top" && environment == CommonAotEnvironment::SealedProgram
        {
            let mut cells = Vec::new();
            if let ClosedProgramCoverageDecision::Selected(coverage) = closed_program_coverage {
                for statement in &coverage.statements {
                    if let ClosedProgramStatementEvidence::DirectActualConstant { value, .. } =
                        statement
                    {
                        let cell = function.ssa.cell_key(value.symbol);
                        if !cells.contains(cell) {
                            cells.push(cell.clone());
                        }
                    }
                }
            }
            cells.truncate(crate::var_escape::LOCALS_ARRAY_CAP);
            cells
        } else {
            Vec::new()
        };
        let local_slots = summary
            .map(|summary| summary.local_slots.clone())
            .unwrap_or_default();
        // The lowered module's own numeral grammar (`Module::dialect`) — the
        // same source `codegen_module` reads, so a slot decision made here and
        // the code emitted for it agree on what `0755` is.
        let intervals = compute_intervals_with(
            &function.cfg,
            &function.ssa,
            &function.sccp.values,
            unit.ir_module.number_syntax(),
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
                sealed_cells: &sealed_cells,
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
    sealed_cells: &'a [crate::var_resolve::VariableCellKey],
    propagated: &'a HashMap<(String, usize), Option<TypeLattice>>,
    intervals: &'a HashMap<ValueKey, Interval>,
    enabled: bool,
    environment: CommonAotEnvironment,
}

fn native_frame_slot_allocation(
    input: &MaterialisableInputs<'_>,
    cell: &crate::var_resolve::VariableCellKey,
) -> Option<(u32, MaterialisableSlotAuthority)> {
    use crate::var_escape::original_slots::OriginalNativeFrameSlot;
    // The entry layout belongs only to the root's actual active frame.
    // It cannot be lent to a separately declared procedure.
    if input.qname != input.unit.top_level.name {
        return None;
    }
    let receipt = OriginalNativeFrameSlot::from_entry(&input.unit.ir_module, cell)?;
    let points = input.function.ssa.point_contexts.as_ref()?;
    let mut represented = false;
    for (&block, statements) in &input.function.ssa.blocks {
        for (index, statement) in statements.statements.iter().enumerate() {
            if statement.uses.contains_key(&input.key.0)
                || statement.defs.contains_key(&input.key.0)
            {
                let context = points.context_before(block, index)?;
                // A compiled name alone establishes neither link absence nor
                // quiet activation contents. Those domains remain separate.
                if !receipt.matches_current_activation(context)
                    || !context.activation_observers_closed()
                    || context.dynamic_bindings
                    || context.dynamic_traces
                    || !context.alias_bindings.is_empty()
                    || !context.name_alias_bindings.is_empty()
                {
                    return None;
                }
                represented = true;
            }
        }
    }
    represented.then(|| {
        (
            receipt.ordinal(),
            MaterialisableSlotAuthority::NativeFrame(Box::new(receipt)),
        )
    })
}

fn materialisable_slot_allocation(
    input: &MaterialisableInputs<'_>,
) -> Option<(u32, MaterialisableSlotAuthority)> {
    use crate::var_escape::original_slots::OriginalDeclaredProcedureArgumentSlots;
    use crate::var_resolve::VariableCellKey;
    let cell = input.function.ssa.cell_key(input.key.0);
    if input.qname == input.unit.top_level.name
        && input.unit.ir_module.source_entry.native_entry.is_some()
    {
        return native_frame_slot_allocation(input, cell);
    }
    if input.qname == input.unit.top_level.name
        && input.environment == CommonAotEnvironment::SealedProgram
    {
        if !matches!(
            cell,
            VariableCellKey::Authored(_) | VariableCellKey::Namespace { .. }
        ) {
            return None;
        }
        let ordinal = u32::try_from(
            input
                .sealed_cells
                .iter()
                .position(|candidate| candidate == cell)?,
        )
        .ok()?;
        return Some((
            ordinal,
            MaterialisableSlotAuthority::SealedProgramCell { cell: cell.clone() },
        ));
    }
    match cell {
        VariableCellKey::Authored(_) => {
            let ordinal = *input
                .local_slots
                .get(input.function.ssa.var_name(input.key.0))?;
            Some((ordinal, MaterialisableSlotAuthority::Authored))
        }
        VariableCellKey::Activation { identity, simple } => {
            let procedure = input.unit.ir_module.procedures.get(input.qname)?;
            let declaration = OriginalDeclaredProcedureArgumentSlots::from_module(
                &input.unit.ir_module,
                procedure,
            )?;
            let topology = declaration.topology();
            let ordinal = declaration.arguments().ordinal(simple.as_bytes())?;
            let points = input.function.ssa.point_contexts.as_ref()?;
            let mut represented = false;
            for (&block, statements) in &input.function.ssa.blocks {
                for (index, statement) in statements.statements.iter().enumerate() {
                    if statement.uses.contains_key(&input.key.0)
                        || statement.defs.contains_key(&input.key.0)
                    {
                        let context = points.context_before(block, index)?;
                        if context.activation.as_ref() != Some(identity)
                            || context.original_formal_topology.as_deref() != Some(topology)
                            || context.dynamic_bindings
                            || !context.activation_observers_closed()
                        {
                            return None;
                        }
                        represented = true;
                    }
                }
            }
            represented.then(|| {
                (
                    ordinal,
                    MaterialisableSlotAuthority::SourceFormal {
                        declaration: std::sync::Arc::new(declaration),
                        cell: cell.clone(),
                    },
                )
            })
        }
        _ => None,
    }
}

fn materialisable_decision(input: &MaterialisableInputs<'_>) -> MaterialisableSlotDecision {
    let decline = |reason| MaterialisableSlotDecision::Declined(reason);
    if !input.enabled {
        return decline(MaterialisableSlotDecline::PassDisabled);
    }
    if input.function.requires_native_math_binding_validation() {
        return decline(MaterialisableSlotDecline::MathBindingPrerequisiteRequired);
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
    let Some((local_slot, authority)) = materialisable_slot_allocation(input) else {
        return decline(MaterialisableSlotDecline::OriginalSlotUnavailable);
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
        && let Some(index) = match &authority {
            MaterialisableSlotAuthority::SourceFormal { .. } => usize::try_from(local_slot).ok(),
            MaterialisableSlotAuthority::Authored => {
                proc_def.params.iter().position(|name| name == &variable)
            }
            MaterialisableSlotAuthority::SealedProgramCell { .. }
            | MaterialisableSlotAuthority::NativeFrame(_) => None,
        }
        && let Some(propagated) = input.propagated.get(&(input.qname.to_owned(), index))
    {
        let Some(propagated) = propagated else {
            // A missing actual type on one reached caller is a retained poison
            // fact. Arithmetic use inside the body cannot establish the entry
            // object's representation for that caller.
            return decline(MaterialisableSlotDecline::TypeNotSingleton);
        };
        ty = type_join(&ty, propagated);
    }
    let Some(shape) = ty.single_shape().cloned() else {
        return decline(MaterialisableSlotDecline::TypeNotSingleton);
    };
    MaterialisableSlotDecision::Selected(Box::new(MaterialisableSlotEvidence {
        local_slot,
        variable,
        authority,
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
    }))
}

#[cfg(test)]
mod tests {
    #[test]
    fn direct_formals_consume_native_activation_plan() {
        use tcl_dialect::ParameterGrammar;
        for grammar in [ParameterGrammar::Tcl, ParameterGrammar::Jim] {
            assert!(super::direct_formal_bindings("x y", 2, grammar).is_ok());
            assert_eq!(
                super::direct_formal_bindings("x x", 2, grammar),
                Err(super::DirectProcDecline::SharedFormalSlotUnsupported)
            );
            assert_eq!(
                super::direct_formal_bindings("x y", 1, grammar),
                Err(super::DirectProcDecline::ArityMismatch {
                    expected: 2,
                    actual: 1
                })
            );
            assert_eq!(
                super::direct_formal_bindings("x args", 2, grammar),
                Err(super::DirectProcDecline::VariadicUnsupported)
            );
        }
        assert!(super::direct_formal_bindings("args x", 2, ParameterGrammar::Tcl).is_ok());
        assert_eq!(
            super::direct_formal_bindings("args x", 2, ParameterGrammar::Jim),
            Err(super::DirectProcDecline::VariadicUnsupported)
        );
        assert_eq!(
            super::direct_formal_bindings("&x", 1, ParameterGrammar::Jim),
            Err(super::DirectProcDecline::ReferenceArgumentUnsupported)
        );
        assert!(super::direct_formal_bindings("&x", 1, ParameterGrammar::Tcl).is_ok());
    }

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
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        let unit = native_unit(source, std::sync::Arc::clone(&context));
        CommonAotProofPlan::build_with_retained_metadata(
            &unit,
            context.commands(),
            config,
            environment,
        )
    }

    fn native_unit(
        source: &str,
        context: std::sync::Arc<tcl_registry::model::ContextRegistry>,
    ) -> crate::environment_ingress::RetainedNativeUnit {
        let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
        let (owner, captured) =
            crate::environment_ingress::captured_native_entry_with_owner(profile);
        let entry = crate::command_binding::SourceAnalysisEntry {
            native_entry: Some(std::sync::Arc::new(captured)),
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
            native_compilation: crate::environment_ingress::authoring_native_compilation(),
            ..Default::default()
        };
        let unit = CompilationUnit::build_with_context_registry(
            source,
            crate::compilation_unit::UnitBuildOptions {
                registry: context.commands(),
                defer_top_level: false,
                config: tcl_lexer::LexerConfig::for_dialect("tcl9.0"),
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            Some(&entry),
            std::sync::Arc::clone(&context),
        );
        crate::environment_ingress::RetainedNativeUnit::new(unit, owner)
    }

    fn logical_unit(
        source: &str,
        context: std::sync::Arc<tcl_registry::model::ContextRegistry>,
    ) -> CompilationUnit {
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::clone(&context),
            config,
        );
        let entry = crate::command_binding::SourceAnalysisEntry::for_logical_source(&input)
            .expect("positive supplied Logical source input");
        CompilationUnit::build_with_analysis_input(
            source,
            crate::compilation_unit::UnitBuildOptions {
                registry: context.commands(),
                defer_top_level: false,
                config,
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            Some(&entry),
            &input,
        )
    }

    /// An explicit authored compiler/naming model, without an entered Native world.
    fn authored_program_unit(
        source: &str,
        context: std::sync::Arc<tcl_registry::model::ContextRegistry>,
    ) -> CompilationUnit {
        let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
        let entry = crate::command_binding::SourceAnalysisEntry {
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
            native_compilation: crate::environment_ingress::authoring_native_compilation(),
            ..Default::default()
        };
        assert_eq!(
            entry
                .options()
                .execution_name_policy()
                .unwrap()
                .native_recipe()
                .unwrap()
                .authority(),
            tcl_syntax::naming::NamePolicyAuthority::AuthoredSimulation,
        );
        CompilationUnit::build_with_context_registry(
            source,
            crate::compilation_unit::UnitBuildOptions {
                registry: context.commands(),
                defer_top_level: false,
                config: tcl_lexer::LexerConfig::for_dialect("tcl9.0"),
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            Some(&entry),
            std::sync::Arc::clone(&context),
        )
    }

    #[test]
    fn authored_program_storage_keeps_original_coverage_without_native_frame_or_body_grants() {
        // naming.variable.aot-original-slot-purpose
        // docs/design/analysis/name-resolution-proofs/aot-original-slot-purpose.md
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        let unit = authored_program_unit(
            "proc p {x y} {}; set d 2; set e 4; puts [p $d $e]",
            std::sync::Arc::clone(&context),
        );
        assert!(unit.ir_module.source_entry.native_entry.is_none());
        let plan = CommonAotProofPlan::build_with_retained_metadata(
            &unit,
            context.commands(),
            enabled(),
            CommonAotEnvironment::SealedProgram,
        );
        let ClosedProgramCoverageDecision::Selected(coverage) = plan.closed_program_coverage()
        else {
            panic!("authored original statement coverage absent: {plan:#?}");
        };
        assert_eq!(coverage.statements.len(), 4);
        for variable in ["d", "e"] {
            assert!(plan.materialisable_slots().any(|(identity, decision)| {
                identity.function == "::top" && matches!(decision,
                    MaterialisableSlotDecision::Selected(evidence)
                        if evidence.variable == variable && evidence.shape == TypeShape::Int
                        && matches!(evidence.authority, MaterialisableSlotAuthority::SealedProgramCell { .. })
                        && evidence.runtime_guards.variable_trace_epoch
                        && evidence.runtime_guards.interpreter_policy_epoch)
            }), "authored storage for {variable}: {plan:#?}");
        }
        let direct = plan
            .direct_calls()
            .find_map(|(_, decision)| match decision {
                DirectProcDecision::Selected(direct) if direct.callee.qualified_name == "::p" => {
                    Some(direct)
                }
                _ => None,
            })
            .expect("original empty procedure call retains its independent declaration");
        assert!(matches!(
            direct.body,
            DirectProcBodyDecision::Declined(DirectProcBodyDecline::UnsupportedBodyShape)
        ));
        assert!(!direct.frame_elidable);
        assert!(plan.materialisable_slots().all(|(_, decision)| !matches!(decision,
            MaterialisableSlotDecision::Selected(evidence) if matches!(evidence.authority, MaterialisableSlotAuthority::NativeFrame(_)))));
    }

    #[test]
    fn original_declared_argument_slots_do_not_borrow_the_parent_native_frame() {
        // Implementation contract: naming.variable.aot-original-slot-purpose
        // docs/design/analysis/name-resolution-proofs/aot-original-slot-purpose.md
        use crate::var_escape::original_slots::OriginalDeclaredProcedureArgumentSlots;
        use tcl_syntax::naming::NativeCompiledVariableEnvironment;
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        let unit = native_unit("proc p {x y} {return $x}; p FIRST SECOND", context);
        let module = &unit.ir_module;
        let procedure = &module.procedures["::p"];
        let entry = module.source_entry.native_entry.as_deref().unwrap();
        assert!(entry.compiled_local_layout.is_none());
        let receipt = OriginalDeclaredProcedureArgumentSlots::from_module(module, procedure)
            .expect("original declaration allocates its own argument convention");
        assert_eq!(receipt.arguments().ordinal(b"x"), Some(0));
        assert_eq!(receipt.arguments().ordinal(b"y"), Some(1));
        assert_eq!(receipt.declaration_span(), procedure.span);
        assert!(
            receipt
                .protocol()
                .supports_environment(NativeCompiledVariableEnvironment::DeclareProcedure)
        );
        assert_eq!(
            receipt.arguments().original_input(),
            receipt.topology().original_input()
        );
        let mut entered = module.clone();
        entered.source_entry.compilation_scope =
            tcl_runtime_api::SourceCompilationScope::EnteredSource;
        assert!(OriginalDeclaredProcedureArgumentSlots::from_module(&entered, procedure).is_none());
        let mut unavailable = module.clone();
        std::sync::Arc::make_mut(unavailable.source_entry.native_entry.as_mut().unwrap())
            .compiled_variable_protocol = None;
        assert!(
            OriginalDeclaredProcedureArgumentSlots::from_module(&unavailable, procedure).is_none()
        );
        let mut missing = module.clone();
        missing.procedure_implementation_bodies = std::sync::Arc::from([]);
        assert!(OriginalDeclaredProcedureArgumentSlots::from_module(&missing, procedure).is_none());
        let mut stale = module.clone();
        stale.source = tcl_lexer::SourceImage::document("proc p {x y} {return OTHER}");
        assert!(OriginalDeclaredProcedureArgumentSlots::from_module(&stale, procedure).is_none());
        let mut changed = procedure.clone();
        changed.body.executed_source = None;
        assert!(OriginalDeclaredProcedureArgumentSlots::from_module(module, &changed).is_none());
    }

    #[test]
    fn original_procedure_body_preflight_keeps_entry_and_missing_provider_independent() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // Compiler traversal only: no body execution, numeric result, old frame
        // slot or direct-body specialisation is certified by this control.
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
        let config = tcl_lexer::LexerConfig::for_dialect("tcl9.0");
        for (formals, body, arguments) in [
            ("x", "return $x", "VALUE"),
            ("x y", "return [expr {$x+$y}]", "1 2"),
            ("", "return [expr {1+2}]", ""),
            ("x", "set y $x; return $y", "VALUE"),
        ] {
            let source = format!("proc p {{{formals}}} {{{body}}}; p {arguments}");
            let unit = native_unit(&source, std::sync::Arc::clone(&context));
            let procedure = &unit.ir_module.procedures["::p"];
            let original = procedure.body.executed_source.as_ref().unwrap();
            assert_eq!(original.text.bytes(), body.as_bytes());
            assert!(
                !crate::native_compilation_admission::script_requires_admission(&procedure.body),
                "{body}: {:?}",
                procedure.body.native_compilation_admission,
            );
            let unavailable = crate::command_binding::SourceCommandBindings::analyse_with_options(
                body,
                config,
                context.commands(),
                crate::command_binding::SourceAnalysisOptions {
                    unknown_entry: true,
                    invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                    native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                        mode: tcl_registry::native_compilation::NativeCompilationMode::BytecodeObject,
                        frame: tcl_registry::native_compilation::NativeCompilationFrame::ScriptCode,
                        ..Default::default()
                    },
                    ..Default::default()
                },
            );
            assert!(unavailable.native_compilation_source_at(0).is_some());
            assert!(
                unavailable.native_compilation_provider_required_at(0),
                "{body}"
            );
        }
    }

    #[test]
    fn original_empty_procedure_direct_call_keeps_formals_without_body_specialisation() {
        // Implementation contract: naming.variable.aot-original-slot-purpose
        // docs/design/analysis/name-resolution-proofs/aot-original-slot-purpose.md
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        let unit = native_unit("proc p {x} {}; p VALUE", std::sync::Arc::clone(&context));
        let procedure = &unit.ir_module.procedures["::p"];
        assert!(procedure.body.statements.is_empty());
        assert!(!crate::native_compilation_admission::script_requires_admission(&procedure.body));
        let plan = CommonAotProofPlan::build_with_retained_metadata(
            &unit,
            context.commands(),
            enabled(),
            CommonAotEnvironment::Hosted,
        );
        let selected = plan
            .direct_calls()
            .find_map(|(_, decision)| match decision {
                DirectProcDecision::Selected(evidence)
                    if evidence.callee.qualified_name == "::p" =>
                {
                    Some(evidence)
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("original empty procedure direct call: {plan:#?}"));
        assert_eq!(selected.formals, ["x"]);
        assert_eq!(
            selected.original_formals.as_ref().unwrap().ordinal(b"x"),
            Some(0)
        );
        assert!(matches!(
            selected.body,
            DirectProcBodyDecision::Declined(DirectProcBodyDecline::UnsupportedBodyShape)
        ));
        assert!(!selected.frame_elidable);
    }

    #[test]
    fn retained_aot_metadata_cannot_borrow_missing_or_foreign_function_input() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let registry = tcl_registry::CommandRegistry::build_default();
        let unit = CompilationUnit::build_for_dialect(
            "proc p {x} {return $x}; p VALUE",
            &registry,
            false,
            "tcl8.6",
        );
        let plan = CommonAotProofPlan::build_with_retained_metadata(
            &unit,
            &registry,
            enabled(),
            CommonAotEnvironment::Hosted,
        );
        assert_eq!(plan.direct_calls().count(), 1);
        assert!(plan.direct_calls().all(|(_, decision)| !matches!(
            decision,
            DirectProcDecision::Declined(DirectProcDecline::ContextUnavailable)
        )));
        let mut missing_module = unit.clone();
        missing_module.ir_module.source_metadata_input = None;
        let mut missing_caller = unit.clone();
        missing_caller.top_level.source_metadata_input = None;
        let mut missing_callee = unit.clone();
        missing_callee
            .procedures
            .get_mut("::p")
            .unwrap()
            .source_metadata_input = None;
        let mut stale_grammar = unit.clone();
        stale_grammar.top_level.source_config.expand_syntax =
            !stale_grammar.top_level.source_config.expand_syntax;
        let input = unit.top_level.source_metadata_input().unwrap();
        let mut foreign = unit.clone();
        foreign.top_level.source_metadata_input =
            Some(crate::analyser::ResolvedAnalysisInput::new(
                input.analyser_profile(),
                input.unit_profile(),
                tcl_registry::model::ingress::resolve_environment("tcl9.1")
                    .default_context_registry(),
                input.lexer_config(),
            ));
        for refused in [
            missing_module,
            missing_caller,
            missing_callee,
            stale_grammar,
            foreign,
        ] {
            let plan = CommonAotProofPlan::build_with_retained_metadata(
                &refused,
                &registry,
                enabled(),
                CommonAotEnvironment::Hosted,
            );
            assert_eq!(plan.direct_calls().count(), 1);
            assert!(plan.direct_calls().all(|(_, decision)| matches!(
                decision,
                DirectProcDecision::Declined(DirectProcDecline::ContextUnavailable)
            )));
        }
    }

    #[test]
    fn original_source_queries_require_unanimous_statement_carriers() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        let unit = logical_unit("set result VALUE", context);
        let (block, index) = unit
            .top_level
            .ssa
            .blocks
            .iter()
            .find_map(|(id, block)| {
                block
                    .statements
                    .iter()
                    .enumerate()
                    .find_map(|(index, statement)| {
                        matches!(statement.statement, Statement::AssignConst { .. })
                            .then_some((*id, index))
                    })
            })
            .expect("original source store retains a structural statement");
        let id = DirectCallSiteId {
            function: "::top".into(),
            block,
            statement_index: u32::try_from(index).unwrap(),
            nested_argument: None,
        };
        let statement = &unit.top_level.ssa.blocks[&block].statements[index].statement;
        assert!(
            statement.tokens().is_none(),
            "typed store consumes its inline vector"
        );
        assert_eq!(
            direct_call_tokens(&unit, &id).unwrap().argv_texts,
            ["set", "result", "VALUE"],
        );
        let mut missing = unit.clone();
        missing.top_level.cfg.command_binding_sites.clear();
        assert!(direct_call_tokens(&missing, &id).is_none());
        let mut conflicting = unit.clone();
        let mut other = conflicting
            .top_level
            .cfg
            .command_binding_sites
            .iter()
            .find(|site| site.span == statement.span() && site.source_tokens.is_some())
            .unwrap()
            .clone();
        other.source_tokens.as_mut().unwrap().argv_texts[0] = "other".into();
        conflicting.top_level.cfg.command_binding_sites.push(other);
        assert!(direct_call_tokens(&conflicting, &id).is_none());
    }

    #[test]
    fn structural_call_candidates_keep_original_nested_words_and_literal_boundaries() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        let unit = logical_unit(
            "proc p {x} {return $x}; proc q {x} {return [p $x]}; list {[p VALUE]}",
            context,
        );
        let function = &unit.procedures["::q"];
        let candidates = call_sites("::q", function, function.source_lexer_config());
        let nested = candidates
            .iter()
            .find(|site| site.command == "p")
            .expect("structural return retains its original nested call");
        assert_eq!(nested.id.nested_argument, Some(0));
        assert_eq!(nested.args, ["$x"]);
        assert_eq!(nested.tokens.as_ref().unwrap().argv_texts, ["p", "$x"]);
        let top = call_sites(
            "::top",
            &unit.top_level,
            unit.top_level.source_lexer_config(),
        );
        assert!(
            top.iter().all(|site| site.id.nested_argument.is_none()),
            "procedure body words and braced list values are data at these invocations"
        );
        let mut missing = function.clone();
        missing.cfg.command_binding_sites.clear();
        for block in missing.cfg.blocks.values_mut() {
            for statement in &mut block.statements {
                if let Statement::Return { tokens, .. } = statement {
                    *tokens = None;
                } else {
                    assert!(statement.tokens().is_none());
                }
            }
        }
        assert!(
            call_sites("::q", &missing, missing.source_lexer_config())
                .iter()
                .all(|site| site.command != "p")
        );
    }

    #[test]
    fn original_direct_expression_body_requires_the_actual_nested_operation_receipts() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        let registry = context.commands();
        let unit = logical_unit(
            "proc p {x y} {return [expr {$x+$y}]}",
            std::sync::Arc::clone(&context),
        );
        let function = &unit.procedures["::p"];
        let procedure = &unit.ir_module.procedures["::p"];
        let metadata = function.invocation_metadata_context_for_module(registry, &unit.ir_module);
        let operations = original_direct_expression_body_operations(
            registry,
            metadata,
            procedure,
            function.source_lexer_config(),
        );
        assert_eq!(
            operations,
            Some([
                SemanticOperationId::StructuredLowering(LoweringHookId::Expr),
                SemanticOperationId::StructuredLowering(LoweringHookId::Return),
            ])
        );
        assert!(
            original_direct_expression_body_operations(
                registry,
                None,
                procedure,
                function.source_lexer_config(),
            )
            .is_none()
        );
        let mut missing_source = procedure.clone();
        missing_source.body.command_binding_sites = crate::ir::CommandBindingSites::default();
        for statement in &mut missing_source.body.statements {
            if let Statement::Return { tokens, .. } = statement {
                *tokens = None;
            }
        }
        assert!(
            original_direct_expression_body_operations(
                registry,
                metadata,
                &missing_source,
                function.source_lexer_config(),
            )
            .is_none()
        );
        for source in [
            "proc expr args {return 0}; proc p {x y} {return [expr {$x+$y}]}",
            "proc return args {}; proc p {x y} {return [expr {$x+$y}]}",
            "proc p {x y} {return [expr {$x+[unknown]}]}",
        ] {
            let changed = logical_unit(source, std::sync::Arc::clone(&context));
            let function = &changed.procedures["::p"];
            assert!(
                original_direct_expression_body_operations(
                    registry,
                    function.invocation_metadata_context_for_module(registry, &changed.ir_module),
                    &changed.ir_module.procedures["::p"],
                    function.source_lexer_config(),
                )
                .is_none(),
                "{source}"
            );
        }
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
    fn direct_procedure_selection_preserves_native_preformal_admission() {
        use crate::native_compilation_admission::NativeCompilationAdmission;
        use std::sync::Arc;
        let registry = tcl_registry::CommandRegistry::build_default();
        let mut unit = CompilationUnit::build_for_dialect(
            "proc p {} {return 1}; p",
            &registry,
            false,
            "tcl9.0",
        );
        unit.ir_module
            .procedures
            .get_mut("::p")
            .unwrap()
            .body
            .native_compilation_admission = Some(Arc::new(NativeCompilationAdmission {
            source: None,
            failure: None,
            provider_required: true,
        }));
        let plan = CommonAotProofPlan::build_with_retained_metadata(
            &unit,
            &registry,
            enabled(),
            CommonAotEnvironment::Hosted,
        );
        assert!(plan.direct_calls().any(|(_, decision)| matches!(
            decision,
            DirectProcDecision::Declined(DirectProcDecline::NativeCompilationAdmissionRequired)
        )));
    }

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

    fn original_formal_diagnostics(unit: &CompilationUnit, function: &str) -> String {
        let function = &unit.procedures[function];
        let mut rows = vec![format!("SSA cells: {:?}", function.ssa.cell_keys())];
        if let Some(points) = &function.ssa.point_contexts {
            for (&block, body) in &function.cfg.blocks {
                for index in (0..body.statements.len()).chain(std::iter::once(usize::MAX)) {
                    if let Some(context) = points.context_before(block, index) {
                        rows.push(format!(
                            "{block:?}/{index}: activation={:?}, topology={}, observers={}, dynamic={}",
                            context.activation,
                            context.original_formal_topology.is_some(),
                            context.activation_observers_closed(),
                            context.dynamic_bindings,
                        ));
                    }
                    for access in points.source_reads_at(block, index) {
                        rows.push(format!(
                            "read {:?} at {:?}: residual={:?}, activations={:?}",
                            access.original_spelling,
                            access.source.span,
                            access.context_residual(),
                            access
                                .context_alternatives()
                                .iter()
                                .map(|context| (
                                    &context.activation,
                                    context.original_formal_topology.is_some(),
                                    context.activation_observers_closed(),
                                    context.dynamic_bindings,
                                ))
                                .collect::<Vec<_>>(),
                        ));
                    }
                }
            }
        }
        rows.join("\n")
    }

    fn assert_declared_add_arguments(
        plan: &CommonAotProofPlan,
        unit: &CompilationUnit,
        direct: &DirectProcEvidence,
    ) {
        let selected: Vec<_> = plan
            .declared_arguments()
            .filter_map(|(identity, decision)| match decision {
                DeclaredArgumentDecision::Selected(evidence)
                    if identity.procedure == direct.callee =>
                {
                    Some((identity, evidence))
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            selected.len(),
            2,
            "{}\n{plan:#?}",
            original_formal_diagnostics(&unit, "::add")
        );
        assert!(
            selected
                .iter()
                .all(|(_, slot)| slot.shape() == &TypeShape::Int)
        );
        assert!(selected.iter().all(|(_, slot)| {
            slot.storage() == VarStorage::MaterializableSlot
                && slot.recipe()
                    == MaterialisationRecipe::RetainOriginalTclObject {
                        sharing: SharingState::Shared,
                    }
        }));
        assert!(
            plan.materialisable_slots().all(|(identity, decision)| {
                identity.function != "::add"
                    || !matches!(decision, MaterialisableSlotDecision::Selected(_))
            }),
            "declared arguments cannot fabricate a physical SSA value"
        );
        assert!(selected.iter().all(|(identity, slot)| {
            slot.reads().iter().all(|read| {
                plan.declared_argument_for_incoming_read(&direct.callee, read)
                    .is_some_and(|(selected, _)| selected == *identity)
            })
        }));
    }

    #[test]
    fn add_call_and_integer_formals_receive_common_proofs() {
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        let unit = native_unit(ADD, std::sync::Arc::clone(&context));
        let plan = CommonAotProofPlan::build_with_retained_metadata(
            &unit,
            context.commands(),
            enabled(),
            CommonAotEnvironment::Hosted,
        );
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

        assert_declared_add_arguments(&plan, &unit, direct);

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
    fn declared_arguments_keep_distinct_reached_cells_and_exact_source_receipts() {
        // naming.variable.aot-original-slot-purpose
        // docs/design/analysis/name-resolution-proofs/aot-original-slot-purpose.md
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        let unit = native_unit(
            "proc p {x y} {return [expr {$x + $y}]}; p 1 2; p 3 4",
            std::sync::Arc::clone(&context),
        );
        let plan = CommonAotProofPlan::build_with_retained_metadata(
            &unit,
            context.commands(),
            enabled(),
            CommonAotEnvironment::Hosted,
        );
        let arguments: Vec<_> = plan
            .declared_arguments()
            .filter_map(|(identity, decision)| {
                let DeclaredArgumentDecision::Selected(evidence) = decision else {
                    return None;
                };
                Some((identity, evidence))
            })
            .collect();
        assert_eq!(arguments.len(), 2, "{plan:#?}");
        assert_eq!(
            arguments
                .iter()
                .map(|(identity, _)| identity.ordinal)
                .collect::<Vec<_>>(),
            [0, 1]
        );
        assert!(
            arguments
                .iter()
                .all(|(_, evidence)| evidence.reads().iter().any(|read| read.cells.len() >= 2))
        );
        assert!(
            plan.materialisable_slots()
                .all(|(identity, decision)| identity.function != "::p"
                    || !matches!(decision, MaterialisableSlotDecision::Selected(_)))
        );
        let (identity, evidence) = arguments[0];
        let read = &evidence.reads()[0];
        assert!(
            plan.declared_argument_for_incoming_read(&identity.procedure, read)
                .is_some()
        );
        let mut wrong_source = read.clone();
        wrong_source.source.span =
            tcl_lexer::Span::new(read.source.span.start() + 1, read.source.span.end());
        let mut wrong_cell = read.clone();
        wrong_cell.cells.pop();
        let mut wrong_slot = read.clone();
        wrong_slot.slot = "y".into();
        for invalid in [wrong_source, wrong_cell, wrong_slot] {
            assert!(
                plan.declared_argument_for_incoming_read(&identity.procedure, &invalid)
                    .is_none()
            );
        }
        let mut wrong_declaration = identity.procedure.clone();
        wrong_declaration.definition_start += 1;
        assert!(
            plan.declared_argument_for_incoming_read(&wrong_declaration, read)
                .is_none()
        );
    }

    #[test]
    fn declared_arguments_refuse_missing_owners_changed_contents_and_future_callers() {
        // naming.variable.aot-original-slot-purpose
        // docs/design/analysis/name-resolution-proofs/aot-original-slot-purpose.md
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        let unit = native_unit(
            "proc p {x} {return [expr {$x + 0}]}; p 1",
            std::sync::Arc::clone(&context),
        );
        let make_plan = |unit: &CompilationUnit| {
            CommonAotProofPlan::build_with_retained_metadata(
                unit,
                context.commands(),
                enabled(),
                CommonAotEnvironment::Hosted,
            )
        };
        assert!(
            make_plan(&unit)
                .declared_arguments()
                .any(|(_, decision)| matches!(decision, DeclaredArgumentDecision::Selected(_)))
        );
        let mut missing = (*unit).clone();
        missing.ir_module.source_metadata_input = None;
        let mut changed = (*unit).clone();
        changed.ir_module.source =
            tcl_lexer::SourceImage::document("proc p {x} {return DIFFERENT}; p 1");
        for refused in [&missing, &changed] {
            assert!(
                make_plan(refused)
                    .declared_arguments()
                    .all(|(_, decision)| matches!(decision, DeclaredArgumentDecision::Declined(_)))
            );
        }
        for source in [
            "proc p {x} {set x 9; return [expr {$x + 0}]}; p 1",
            "proc p {x} {trace add variable x read {list}; return [expr {$x + 0}]}; p 1",
            "proc p {x} {return [expr {$x + 0}]}; proc future {y} {p $y}; p 1",
        ] {
            let unit = native_unit(source, std::sync::Arc::clone(&context));
            let plan = make_plan(&unit);
            assert!(
                plan.declared_arguments()
                    .all(
                        |(identity, decision)| identity.procedure.qualified_name != "::p"
                            || matches!(decision, DeclaredArgumentDecision::Declined(_))
                    ),
                "{source}: {plan:#?}"
            );
        }
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
                SemanticCallDecline::ReboundOrAliased,
            ),
            (
                "proc add {b c} {return [expr {$b+$c}]}\nrename puts coreputs\ninterp alias {} puts {} coreputs\nputs [add 2 4]\n",
                SemanticCallDecline::BindingNotOriginal {
                    kind: BindingKind::Alias,
                },
            ),
            (
                "proc cb args {}; proc add {b c} {return [expr {$b+$c}]}\ntrace add execution puts enter cb\nputs [add 2 4]\n",
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
                MaterialisableSlotDecision::Selected(evidence)
                    if evidence.storage == VarStorage::NativeOnly
            )
        }));
        assert!(CommonAotEnvironment::SealedProgram.permits_native_only());
    }

    #[test]
    fn retained_native_program_coverage_does_not_donate_root_frame_slots() {
        // naming.variable.aot-original-slot-purpose
        // docs/design/analysis/name-resolution-proofs/aot-original-slot-purpose.md
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        let unit = native_unit(ADD, std::sync::Arc::clone(&context));
        assert!(
            unit.ir_module
                .source_entry
                .native_entry
                .as_ref()
                .unwrap()
                .compiled_local_layout
                .is_none()
        );
        let plan = CommonAotProofPlan::build_with_retained_metadata(
            &unit,
            context.commands(),
            enabled(),
            CommonAotEnvironment::SealedProgram,
        );
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
        for variable in ["d", "e"] {
            let decisions: Vec<_> = plan
                .materialisable_slots()
                .filter(|(identity, _)| {
                    identity.function == "::top"
                        && unit.top_level.ssa.var_name(identity.symbol) == variable
                })
                .map(|(_, decision)| decision)
                .collect();
            assert!(
                !decisions.is_empty(),
                "original root {variable} is represented"
            );
            assert!(
                decisions.iter().all(|decision| matches!(
                    decision,
                    MaterialisableSlotDecision::Declined(
                        MaterialisableSlotDecline::OriginalSlotUnavailable
                    )
                )),
                "captured ScriptCode cannot borrow a procedure local: {decisions:#?}"
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
        for (source, expected_operation, transitive_mutation_is_dynamic) in [
            (
                "proc add {b c} {return [expr {$b+$c}]}\nrename expr saved_expr\nadd 2 4\n",
                SemanticOperationId::StructuredLowering(LoweringHookId::Expr),
                true,
            ),
            (
                "proc cb args {}; proc add {b c} {return [expr {$b+$c}]}\ntrace add execution expr enter cb\nadd 2 4\n",
                SemanticOperationId::StructuredLowering(LoweringHookId::Expr),
                false,
            ),
        ] {
            let context = tcl_registry::model::ingress::resolve_environment("tcl9.0")
                .default_context_registry();
            let registry = context.commands();
            let unit = native_unit(source, std::sync::Arc::clone(&context));
            assert_eq!(
                unit.command_mutations.has_dynamic_mutation(),
                transitive_mutation_is_dynamic,
                "the closed effect projection must retain unresolved/autoload opacity"
            );
            assert!(
                unit.caller_scope
                    .proc_binding_trust
                    .trusts_proc_binding("::add"),
                "body-dispatch opacity must not erase the separately proven proc identity"
            );
            let plan = CommonAotProofPlan::build_with_retained_metadata(
                &unit,
                registry,
                enabled(),
                CommonAotEnvironment::Hosted,
            );
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
    fn known_computed_rename_preserves_callee_identity_and_rejects_its_internal_operation() {
        let proof = plan(
            "proc add {b c} {return [expr {$b+$c}]}; set command expr; rename $command saved_expr; add 2 4",
            enabled(),
        );
        assert!(
            proof.direct_calls().any(|(_, decision)| matches!(
                decision,
                DirectProcDecision::Selected(DirectProcEvidence {
                    body: DirectProcBodyDecision::Declined(
                        DirectProcBodyDecline::InternalDispatchUntrusted { .. }
                    ),
                    ..
                })
            )),
            "{proof:#?}"
        );
    }

    #[test]
    fn dynamic_binding_transition_still_declines_direct_call_identity() {
        let plan = plan(
            "proc add {b c} {return [expr {$b+$c}]}\n\
             set command [lindex $argv 0]\nrename $command saved_expr\nadd 2 4\n",
            enabled(),
        );
        assert!(plan.direct_calls().any(|(_, decision)| matches!(
            decision,
            DirectProcDecision::Declined(DirectProcDecline::DynamicCommandMutation)
        )));
    }

    #[test]
    fn dialect_tcloo_and_variable_trace_premises_are_retained() {
        let registry = tcl_registry::CommandRegistry::build_default();
        // An availability set spanning several dialects is not something a
        // production caller can produce, so it is not what this pins.
        // The executable-IR vocabulary is now a resolved
        // environment, which names exactly one context or none, so an
        // ambiguous premise is unrepresentable rather than declined. The
        // decline it pins is therefore the surviving one: a unit built with
        // no dialect carries no context.
        let unit = CompilationUnit::build_for(ADD, &registry, false);
        let ambiguous = CommonAotProofPlan::build_with_retained_metadata(
            &unit,
            &registry,
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
        // naming.variable.aot-original-slot-purpose
        // docs/design/analysis/name-resolution-proofs/aot-original-slot-purpose.md
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        let unit = native_unit(
            "proc add {b c} {return [expr {$b+$c}]}\nproc caller {x} {set d 2; add $d $x}\nset d 2\nset e 4\nadd $d $e\ncaller [eval {set original_input 4}]\n",
            std::sync::Arc::clone(&context),
        );
        let plan = CommonAotProofPlan::build_with_retained_metadata(
            &unit,
            context.commands(),
            enabled(),
            CommonAotEnvironment::Hosted,
        );
        // The second caller is actually entered. Its original eval operand
        // is not a represented SSA actual, so the call's type remains unknown
        // independently of the runtime bytes produced by the original setter.
        // Exact source reads can be retained without a scalar SSA entry key.
        // Join the same selected-caller type owner used by materialisation;
        // absence of a physical formal SSA key cannot be replaced by fake v0.
        let mut propagated = HashMap::new();
        let mut selected_callers = 0;
        for (_, decision) in plan.direct_calls() {
            if let DirectProcDecision::Selected(evidence) = decision
                && evidence.callee.qualified_name == "::add"
            {
                selected_callers += 1;
                propagate_actual_types(&mut propagated, "::add", decision);
            }
        }
        assert_eq!(
            selected_callers,
            2,
            "{}\n{}\n{plan:#?}",
            original_formal_diagnostics(&unit, "::caller"),
            original_formal_diagnostics(&unit, "::add"),
        );
        let actual = propagated
            .get(&("::add".to_owned(), 1))
            .expect("both callers recorded");
        assert!(
            actual
                .as_ref()
                .is_none_or(|value| value.single_shape().is_none())
        );
        assert!(!plan.materialisable_slots().any(|(identity, decision)| {
            identity.function == "::add"
                && matches!(decision, MaterialisableSlotDecision::Selected(_))
        }));
    }

    #[test]
    fn unentered_unknown_caller_keeps_a_refusal_without_fabricating_a_reached_call() {
        // naming.variable.aot-original-slot-purpose
        // docs/design/analysis/name-resolution-proofs/aot-original-slot-purpose.md
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        let unit = native_unit(
            "proc add {b c} {return [expr {$b+$c}]}\nproc caller {x} {set d 2; add $d $x}\nset d 2\nset e 4\nadd $d $e\n",
            std::sync::Arc::clone(&context),
        );
        let plan = CommonAotProofPlan::build_with_retained_metadata(
            &unit,
            context.commands(),
            enabled(),
            CommonAotEnvironment::Hosted,
        );
        assert_eq!(plan.direct_calls().filter(|(_, decision)| matches!(decision,
            DirectProcDecision::Selected(evidence) if evidence.callee.qualified_name == "::add"
        )).count(), 1, "{plan:#?}");
        assert!(
            plan.direct_calls()
                .any(|(site, decision)| site.function == "::caller"
                    && matches!(
                        decision,
                        DirectProcDecision::Declined(DirectProcDecline::BindingNotProcedure {
                            kind: BindingKind::Unknown
                        })
                    )),
            "future conditional lookup cannot grant reached procedure dispatch: {plan:#?}"
        );
        assert!(
            plan.declared_arguments()
                .any(
                    |(identity, decision)| identity.procedure.qualified_name == "::add"
                        && identity.ordinal == 1
                        && matches!(
                            decision,
                            DeclaredArgumentDecision::Declined(
                                MaterialisableSlotDecline::TypeNotSingleton
                            )
                        )
                ),
            "unresolved source caller remains an independent poison premise: {plan:#?}"
        );
    }

    #[test]
    fn direct_calls_retain_native_argv_expansion_at_the_generic_boundary() {
        let proof = plan("proc p {x y} {return $x}; p {*}{2 4}", enabled());
        assert!(
            proof.direct_calls().any(|(_, decision)| matches!(
                decision,
                DirectProcDecision::Declined(DirectProcDecline::ExpandedArgumentsUnsupported)
            )),
            "{proof:#?}"
        );
        assert!(proof.has_declined_direct_call_to("::p"));
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
