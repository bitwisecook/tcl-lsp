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

//! Direct procedure calls: the evidence and decline types, formal binding, body specialisation and the collector.

use super::semantic_calls::semantic_operation_binding_is_trusted;
use super::{
    AotMetadataSelection, CallCandidate, DirectCallSiteId, ProcedureIdentity, SsaValueIdentity,
    call_sites, function_units, nested_call_tokens,
};
use crate::command_binding::BindingKind;
use crate::compilation_unit::{CompilationUnit, FunctionUnit};
use crate::ir::{CommandTokens, Procedure, Statement};
use crate::registry_invocation::InvocationMetadataContext;
use crate::semantic_optimisation::{SemanticOptimisationConfig, SemanticOptimisationPassId};
use crate::types::{TypeKind, TypeLattice, type_join};
use crate::var_escape::ProcEscapeSummary;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use tcl_registry::hooks::LoweringHookId;
use tcl_registry::model::semantic::SemanticContext;
use tcl_registry::{DispatchDependencies, DispatchDependencyDomain, SemanticOperationId};

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
    Selected(Box<DirectProcEvidence>),
    /// The generic Tcl invocation remains required.
    Declined(DirectProcDecline),
}

pub(super) struct DirectCollection {
    pub(super) decisions: BTreeMap<DirectCallSiteId, DirectProcDecision>,
    /// Callees named by at least one *declined* candidate — the set a consumer
    /// joining facts across selected sites must consult before treating those
    /// sites as the callee's complete caller set.
    pub(super) declined_callees: BTreeSet<crate::ir::ProcedureId>,
    pub(super) propagated: HashMap<(crate::ir::ProcedureId, usize), Option<TypeLattice>>,
}

pub(super) fn collect_direct_calls(
    unit: &CompilationUnit,
    registry: &tcl_registry::CommandRegistry,
    selection: AotMetadataSelection,
    config: SemanticOptimisationConfig,
    escape: &HashMap<tcl_core_types::BodyFrameId, ProcEscapeSummary>,
    mutations: &crate::command_binding::ModuleCommandMutations,
    proc_binding_trust: &crate::command_binding::ProcBindingTrustProjection,
) -> DirectCollection {
    let mut direct_calls = BTreeMap::new();
    let mut declined_callees = BTreeSet::new();
    // `None` is a poison fact: at least one selected call supplied an actual
    // whose type was not proved, or the callee has an unselected caller (see
    // the declined-callee sweep below). `TypeLattice::Unknown` is lattice
    // bottom, so joining it directly would incorrectly preserve another call's
    // fact.
    let mut propagated: HashMap<(crate::ir::ProcedureId, usize), Option<TypeLattice>> =
        HashMap::new();
    for (caller_name, function) in function_units(unit) {
        for site in call_sites(unit, caller_name, function, function.source_lexer_config()) {
            let binding = site
                .tokens
                .as_deref()
                .and_then(|tokens| tokens.source_binding.as_ref());
            let terminal = binding
                .and_then(crate::command_binding::SourceInvocationBinding::proved_execution_target);
            let target = binding.and_then(|binding| binding.direct_procedure_target(&site.command));
            let resolved = terminal
                .and_then(|target| unit.ir_module.source_procedure_for_target(target))
                .or_else(|| {
                    let ids = crate::registry_invocation::original_source_procedure_ids_for_word_for_module(
                        site.tokens.as_deref()?, &site.command, &unit.ir_module, registry,
                    )?;
                    let [id] = ids.as_slice() else { return None; };
                    Some((*id, unit.ir_module.procedures.get(id)?))
                });
            let Some((callee_id, proc_def)) = resolved else {
                continue;
            };
            let callee_name = proc_def.qualified_name.clone();
            let decision = direct_decision(&DirectInputs {
                unit,
                registry,
                function,
                site: &site,
                binding_kind: terminal.map_or(BindingKind::Unknown, |target| target.kind),
                binding_target: terminal.map(|target| target.command.as_str()),
                source_target: target,
                callee_name: &callee_name,
                callee_id,
                proc_def,
                summary: escape.get(&tcl_core_types::BodyFrameId::Body(callee_id)),
                mutations,
                proc_binding_trust,
                context: selection.standalone_context(),
                metadata: selection.for_function(function, registry, &unit.ir_module),
                callee_metadata: unit
                    .procedures
                    .get(&callee_id)
                    .and_then(|callee| selection.for_function(callee, registry, &unit.ir_module)),
                enabled: config.is_enabled(SemanticOptimisationPassId::DirectProc),
                frame_elision_enabled: config.is_enabled(SemanticOptimisationPassId::FrameElision),
                native_integer_enabled: config
                    .is_enabled(SemanticOptimisationPassId::NativeInteger),
            });
            propagate_actual_types(&mut propagated, callee_id, &decision);
            if matches!(decision, DirectProcDecision::Declined(_)) {
                declined_callees.insert(callee_id);
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

pub(super) fn propagate_actual_types(
    propagated: &mut HashMap<(crate::ir::ProcedureId, usize), Option<TypeLattice>>,
    callee_id: crate::ir::ProcedureId,
    decision: &DirectProcDecision,
) {
    let DirectProcDecision::Selected(evidence) = decision else {
        return;
    };
    for (index, ty) in evidence.actual_types.iter().enumerate() {
        propagated
            .entry((callee_id, index))
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

struct DirectInputs<'a> {
    pub(super) unit: &'a CompilationUnit,
    pub(super) registry: &'a tcl_registry::CommandRegistry,
    pub(super) function: &'a FunctionUnit,
    pub(super) site: &'a CallCandidate<'a>,
    binding_kind: BindingKind,
    binding_target: Option<&'a str>,
    source_target: Option<&'a crate::command_binding::SourceCommandTarget>,
    callee_name: &'a str,
    callee_id: crate::ir::ProcedureId,
    proc_def: &'a Procedure,
    pub(super) summary: Option<&'a ProcEscapeSummary>,
    pub(super) mutations: &'a crate::command_binding::ModuleCommandMutations,
    pub(super) proc_binding_trust: &'a crate::command_binding::ProcBindingTrustProjection,
    pub(super) context: Option<SemanticContext>,
    pub(super) metadata: Option<InvocationMetadataContext<'a>>,
    callee_metadata: Option<InvocationMetadataContext<'a>>,
    pub(super) enabled: bool,
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
            .procedures
            .get(&input.callee_id)
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
    if crate::registry_invocation::original_procedure_has_execution_trace(
        input.registry,
        input.metadata,
        input.site.tokens.as_deref(),
        input.callee_name,
        &input.unit.ir_module.traced_commands,
    ) {
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
            .get(&input.callee_id)
            .is_some_and(|callee| !callee.cfg.exception_edges.is_empty())
    {
        return decline(DirectProcDecline::ExceptionalControlFlow);
    }
    if input.site.tokens.as_deref().is_some_and(|tokens| {
        tokens
            .words()
            .iter()
            .any(|word| matches!(word, tcl_source::words::WordExpr::Expand { .. }))
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
        .map(|(index, _)| {
            actual_fact(
                input.function,
                input.site,
                index,
                input.registry,
                &input.unit.ir_module,
            )
        })
        .collect();
    let actual_types = actual_facts.iter().map(|fact| fact.0.clone()).collect();
    let actual_values = actual_facts.into_iter().map(|fact| fact.1).collect();
    let body = direct_body_decision(input, formals);
    let frame_elidable =
        matches!(body, DirectProcBodyDecision::Selected(_)) && summary.safe_for_frame_elision();
    DirectProcDecision::Selected(Box::new(DirectProcEvidence {
        callee: ProcedureIdentity {
            id: input.callee_id,
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
    }))
}

pub(super) fn direct_formal_bindings(
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
#[doc(hidden)]
pub fn original_direct_expression_body_operations(
    registry: &tcl_registry::CommandRegistry,
    metadata: Option<InvocationMetadataContext<'_>>,
    procedure: &Procedure,
    config: tcl_lexer::LexerConfig,
) -> Option<[SemanticOperationId; 2]> {
    let [nested, outer] =
        original_direct_expression_body_invocations(registry, metadata?, procedure, config)?;
    Some([nested.operation, outer.operation])
}

struct DirectBodyInvocation {
    operation: SemanticOperationId,
    tokens: CommandTokens,
    selected: crate::registry_invocation::LogicalStructuredInvocation,
}

fn original_direct_expression_body_invocations(
    registry: &tcl_registry::CommandRegistry,
    metadata: InvocationMetadataContext<'_>,
    procedure: &Procedure,
    config: tcl_lexer::LexerConfig,
) -> Option<[DirectBodyInvocation; 2]> {
    let [statement @ Statement::Return { expr: Some(_), .. }] =
        procedure.body.statements.as_slice()
    else {
        return None;
    };
    let original = procedure
        .body
        .retained_source_tokens_for_statement(statement)?;
    let invocation = |tokens: &CommandTokens, expected| {
        if tokens.words().len() != 2 {
            return None;
        }
        let selected =
            crate::registry_invocation::logical_structured_invocation_with_metadata_context(
                registry, metadata, tokens, None,
            )?;
        (selected.lowering_hook() == Some(expected)).then(|| DirectBodyInvocation {
            operation: SemanticOperationId::StructuredLowering(expected),
            tokens: tokens.clone(),
            selected,
        })
    };
    let outer = invocation(original, LoweringHookId::Return)?;
    let children = crate::word_subst::checked_lifted_calls(original, config)?;
    let [child] = children.as_slice() else {
        return None;
    };
    let nested = invocation(child.tokens.as_ref()?, LoweringHookId::Expr)?;
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
    }
    let Some(callee) = input.unit.function_for_body(Some(input.callee_id)) else {
        return decline(DirectProcBodyDecline::UnsupportedBodyShape);
    };
    let Some(metadata) = input.callee_metadata else {
        return decline(DirectProcBodyDecline::UnsupportedBodyShape);
    };
    let Some(invocations) = original_direct_expression_body_invocations(
        input.registry,
        metadata,
        input.proc_def,
        callee.source_lexer_config(),
    ) else {
        return decline(DirectProcBodyDecline::UnsupportedBodyShape);
    };
    // The trace hazard is the one the body's own original invocation meets,
    // as for a selected semantic call.
    for invocation in &invocations {
        if crate::registry_invocation::original_invocation_has_execution_trace(
            input.registry,
            metadata,
            &invocation.tokens,
            invocation.selected.descriptor_key(),
            &input.unit.ir_module.traced_commands,
        ) {
            return decline(DirectProcBodyDecline::InternalExecutionTrace {
                operation: invocation.operation,
            });
        }
    }
    let operations = invocations.map(|invocation| invocation.operation);
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

pub(super) fn actual_fact(
    function: &FunctionUnit,
    site: &CallCandidate<'_>,
    argument: usize,
    registry: &tcl_registry::CommandRegistry,
    module: &crate::ir::Module,
) -> (TypeLattice, DirectActualValue) {
    if site.id.module_id != module.procedure_ids.module_id() || site.id.body_id != function.body_id
    {
        return (TypeLattice::unknown(), DirectActualValue::Unproven);
    }
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
                module_id: module.procedure_ids.module_id(),
                body_id: function.body_id,
                function: function.name.clone(),
                symbol,
                version,
            }),
        );
    }
    // A literal has source-owned contents without a caller SSA identity.
    // This readonly type does not authorise materialising a Native object.
    let literal = site.tokens.as_deref().and_then(|tokens| {
        crate::type_infer::original_literal_argument_contents(registry, module, tokens, argument)
    });
    (
        literal.map_or_else(TypeLattice::unknown, |literal| literal.contents_type()),
        DirectActualValue::Unproven,
    )
}

pub(super) fn direct_call_tokens<'a>(
    unit: &'a CompilationUnit,
    id: &DirectCallSiteId,
) -> Option<std::borrow::Cow<'a, CommandTokens>> {
    if id.module_id != unit.ir_module.procedure_ids.module_id() {
        return None;
    }
    let function = unit.function_for_body(id.body_id)?;
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
    if id.module_id != unit.ir_module.procedure_ids.module_id() {
        return None;
    }
    let function = unit.function_for_body(id.body_id)?;
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
#[doc(hidden)]
pub fn direct_call_argument_value(
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
