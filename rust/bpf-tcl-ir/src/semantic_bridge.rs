// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Conservative bridge from common Tcl semantic facts to the BPF-Tcl subset.
//!
//! BPF-Tcl has always been a typed Tcl *front end*, rather than a Tcl runtime
//! embedded in eBPF.  Its existing lowerer therefore consumes the
//! registry-owned [`BpfOpSpec`] descriptor and emits BPF-IR directly.  This
//! module is a deliberately disconnected migration adapter: it audits the
//! common [`SemanticAnalysisBundle`] before a future eBPF backend uses those
//! facts for selection.  It does not alter the established frontend, typed
//! lowerer, BPF-IR, or emitter.
//!
//! The common [`SemanticOperationId`] currently has no BPF-Tcl operation
//! variants: BPF command specs resolve to the generic `Invoke` identity.  A
//! generic Tcl invocation is never legal eBPF input — eBPF has no Tcl runtime.
//! Consequently this bridge derives a private, descriptor-based operation key
//! from the registry's `BpfOpSpec`, and only assesses that key *after* it has
//! proved that common completion, transition, callback, and world-effect facts
//! are closed.  Once BPF operations gain shared semantic-operation identities,
//! this adapter can be replaced without changing the BPF lowerer.
//!
//! The explicit BPF dialect selects the sealed syntax and descriptor-dispatch
//! policy while the source is compiled.  A BPF-Tcl program does not run inside
//! a live Tcl interpreter, so it does not need command-binding or trace epoch
//! guards.  That is not permission to ignore common facts: any Tcl-world
//! access, transition, or callback means the source is not a no-runtime eBPF
//! region and must be declined.

use tcl_compiler::executable_ir::InvocationResolution;
use tcl_compiler::registry_invocation::OwnedInvocationResolutionUnresolved;
use tcl_compiler::semantic_analysis::{ExecutableAnalysisAvailability, SemanticAnalysisBundle};
use tcl_compiler::target_contract::{
    LegalisationDecision, LegalisationFailure, LegalisationRequirements, LegalisationStrategy,
    MemoryFeature, OperationLegalisation, SemanticOperationKey, TargetCapabilities, TargetContract,
    TargetFamily, ValueFeature,
};
use tcl_registry::bpf_op::{BpfDeclKind, BpfOpKind, BpfOpSpec};
use tcl_registry::{
    CommandRegistry, CompletionCode, CompletionCodeDomain, CompletionPayloadObligations,
    InvocationFacts, StateTransitionKnowledge,
};

/// A registry-described BPF operation class suitable for eBPF legalisation.
///
/// This is intentionally an adapter-local key.  It is derived exclusively
/// from [`BpfOpSpec::kind`], never from a Tcl command spelling.  It exists
/// because the shared semantic operation catalogue has not yet assigned BPF
/// operation identities; it must not be confused with `SemanticOperationId::Invoke`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EbpfOperationKey {
    /// A fixed-width scalar assignment.
    Scalar,
    /// Packet context or packet-data access.
    PacketAccess,
    /// A map declaration or map lookup/update.
    Map,
    /// An eBPF verdict return.
    Verdict,
}

impl SemanticOperationKey for EbpfOperationKey {}

impl EbpfOperationKey {
    fn from_descriptor(descriptor: BpfOpSpec) -> Result<Self, EbpfOperationKindDecline> {
        match descriptor.kind {
            BpfOpKind::ScalarSet(_) => Ok(Self::Scalar),
            BpfOpKind::BindPacket | BpfOpKind::PacketLoad { .. } | BpfOpKind::PacketLen => {
                Ok(Self::PacketAccess)
            }
            BpfOpKind::MapDeclare | BpfOpKind::MapGet | BpfOpKind::MapSet | BpfOpKind::MapHas => {
                Ok(Self::Map)
            }
            BpfOpKind::Verdict(_) => Ok(Self::Verdict),
            BpfOpKind::LoopMacro => Err(EbpfOperationKindDecline::UnexpandedLoopMacro),
            BpfOpKind::Framework(declaration) => {
                Err(EbpfOperationKindDecline::FrameworkDeclaration(declaration))
            }
        }
    }
}

/// A registry BPF descriptor that is not an executable eBPF operation yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EbpfOperationKindDecline {
    /// `loop` must have been expanded before an executable eBPF region exists.
    UnexpandedLoopMacro,
    /// A framework declaration belongs to frontend processing, not an eBPF body.
    FrameworkDeclaration(BpfDeclKind),
}

/// An immutable, target-independent auditor for candidate BPF-Tcl regions.
///
/// It owns an eBPF target contract only.  There is intentionally no
/// [`tcl_compiler::BackendRegistry`] here: this adapter does not construct a
/// backend plan or invoke an emitter, and registering `Invoke` as a direct
/// eBPF operation would be unsound.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EbpfSemanticBridge {
    target: TargetContract<EbpfOperationKey>,
}

impl Default for EbpfSemanticBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl EbpfSemanticBridge {
    /// Construct the bridge with its fixed eBPF capability contract.
    #[must_use]
    pub fn new() -> Self {
        let mut target = TargetContract::new(TargetFamily::Ebpf, TargetCapabilities::ebpf());
        target.register(
            EbpfOperationKey::Scalar,
            OperationLegalisation::new(LegalisationStrategy::Direct, scalar_requirements()),
        );
        target.register(
            EbpfOperationKey::PacketAccess,
            OperationLegalisation::new(LegalisationStrategy::Direct, packet_requirements()),
        );
        target.register(
            EbpfOperationKey::Map,
            OperationLegalisation::new(LegalisationStrategy::Direct, map_requirements()),
        );
        target.register(
            EbpfOperationKey::Verdict,
            OperationLegalisation::new(LegalisationStrategy::Direct, verdict_requirements()),
        );
        Self { target }
    }

    /// The target contract used for descriptor-derived eBPF operation keys.
    #[must_use]
    pub const fn target(&self) -> &TargetContract<EbpfOperationKey> {
        &self.target
    }

    /// Audit a common semantic bundle as a sealed eBPF candidate.
    ///
    /// A successful result means every executable invocation is a
    /// registry-described BPF operation with an exact normal completion, no
    /// Tcl-world access, no state transition, and no callback/re-entrancy. It
    /// is still only an eligibility record: existing BPF-Tcl lowering and
    /// code generation intentionally do not consume it yet.
    #[must_use]
    pub fn assess(
        &self,
        registry: &CommandRegistry,
        bundle: &SemanticAnalysisBundle,
    ) -> EbpfRegionEligibility {
        let executable = match bundle.executable() {
            ExecutableAnalysisAvailability::Available(facts) => &facts.function,
            ExecutableAnalysisAvailability::ContextUnavailable => {
                return EbpfRegionEligibility::Declined(EbpfRegionDecline::ContextUnavailable);
            }
            ExecutableAnalysisAvailability::WorldStateDeclined { decline, .. } => {
                return EbpfRegionEligibility::Declined(EbpfRegionDecline::WorldStateDeclined(
                    decline.clone(),
                ));
            }
            ExecutableAnalysisAvailability::WorldStateNotRequired { .. } => {
                return EbpfRegionEligibility::Declined(EbpfRegionDecline::WorldStateNotRequested);
            }
            ExecutableAnalysisAvailability::SourceDeclined(decline) => {
                return EbpfRegionEligibility::Declined(EbpfRegionDecline::SourceDeclined(
                    decline.clone(),
                ));
            }
            ExecutableAnalysisAvailability::SourceUnavailable => {
                return EbpfRegionEligibility::Declined(EbpfRegionDecline::SourceUnavailable);
            }
        };

        let shape = audit_executable_shape(executable);
        if !shape.is_empty() {
            return EbpfRegionEligibility::Declined(EbpfRegionDecline::ExecutableShape(shape));
        }

        let mut sealed = Vec::new();
        let mut declines = Vec::new();
        let mut invocation_index = 0;
        for block in &executable.blocks {
            for instruction in &block.instructions {
                let tcl_compiler::executable_ir::ExecutableInstruction::Invoke(invoke) =
                    instruction
                else {
                    continue;
                };
                match self.assess_invocation(registry, invoke, invocation_index) {
                    Ok(invocation) => sealed.push(invocation),
                    Err(decline) => declines.push(decline),
                }
                invocation_index += 1;
            }
        }

        if declines.is_empty() {
            EbpfRegionEligibility::Sealed(SealedEbpfRegion {
                invocations: sealed,
            })
        } else {
            EbpfRegionEligibility::Declined(EbpfRegionDecline::Invocations(declines))
        }
    }

    fn assess_invocation(
        &self,
        registry: &CommandRegistry,
        invoke: &tcl_compiler::executable_ir::GenericInvoke,
        index: usize,
    ) -> Result<SealedEbpfInvocation, EbpfInvocationDecline> {
        let facts = match &invoke.resolution {
            InvocationResolution::Resolved(facts) => facts,
            InvocationResolution::Unresolved(reason) => {
                return Err(EbpfInvocationDecline {
                    index,
                    canonical_command: None,
                    reason: EbpfInvocationDeclineReason::Unresolved(reason.clone()),
                });
            }
        };
        let descriptor = registry.bpf_op(&facts.canonical_command).copied();
        let Some(descriptor) = descriptor else {
            return Err(EbpfInvocationDecline {
                index,
                canonical_command: Some(facts.canonical_command.clone()),
                reason: EbpfInvocationDeclineReason::NotBpfOperation,
            });
        };
        let key = EbpfOperationKey::from_descriptor(descriptor).map_err(|reason| {
            EbpfInvocationDecline {
                index,
                canonical_command: Some(facts.canonical_command.clone()),
                reason: EbpfInvocationDeclineReason::DescriptorKind(reason),
            }
        })?;

        let mut failures = semantic_failures(facts);
        match self.target.assess(&key) {
            LegalisationDecision::Selected(LegalisationStrategy::Direct) => {}
            LegalisationDecision::Selected(other) => {
                failures.push(EbpfSemanticFailure::NonDirectTargetLegalisation(other));
            }
            LegalisationDecision::Declined(reason) => {
                failures.push(EbpfSemanticFailure::TargetLegalisation(reason));
            }
        }
        if !failures.is_empty() {
            return Err(EbpfInvocationDecline {
                index,
                canonical_command: Some(facts.canonical_command.clone()),
                reason: EbpfInvocationDeclineReason::Semantic(failures),
            });
        }

        Ok(SealedEbpfInvocation {
            index,
            canonical_command: facts.canonical_command.clone(),
            operation: key,
        })
    }
}

fn audit_executable_shape(
    function: &tcl_compiler::executable_ir::ExecutableFunction,
) -> Vec<EbpfExecutableShapeDecline> {
    use tcl_compiler::executable_ir::{ExecutableInstruction, ExecutableTerminator};

    let mut declines = Vec::new();
    let mut invocations = 0;
    for block in &function.blocks {
        for (instruction, operation) in block.instructions.iter().enumerate() {
            // This explicit match is intentional. The common executable IR
            // evolves independently; adding an instruction must make this
            // no-runtime adapter choose whether it belongs to the staged
            // argv/invoke shape rather than silently accepting it.
            match operation {
                ExecutableInstruction::EvaluateWord { .. }
                | ExecutableInstruction::ExpandWord { .. }
                | ExecutableInstruction::BuildArgv { .. } => {}
                ExecutableInstruction::Invoke(_) => invocations += 1,
                ExecutableInstruction::ExecuteLowered(_)
                | ExecutableInstruction::ExecuteOpaqueRegion(_) => {
                    declines.push(EbpfExecutableShapeDecline::NonInvocationInstruction {
                        block: block.id.index(),
                        instruction,
                    });
                }
            }
        }
        match &block.terminator {
            Some(
                ExecutableTerminator::CompletionSwitch { .. }
                | ExecutableTerminator::ReturnCompletion(_),
            ) => {}
            Some(ExecutableTerminator::Goto(_)) => {
                declines.push(EbpfExecutableShapeDecline::UnsupportedControlFlow {
                    block: block.id.index(),
                    kind: EbpfControlFlowKind::Goto,
                });
            }
            Some(ExecutableTerminator::Branch { .. }) => {
                declines.push(EbpfExecutableShapeDecline::UnsupportedControlFlow {
                    block: block.id.index(),
                    kind: EbpfControlFlowKind::Branch,
                });
            }
            None => declines.push(EbpfExecutableShapeDecline::UnsupportedControlFlow {
                block: block.id.index(),
                kind: EbpfControlFlowKind::MissingTerminator,
            }),
        }
    }
    if invocations == 0 {
        declines.push(EbpfExecutableShapeDecline::NoAuditedInvocations);
    }
    declines
}

fn scalar_requirements() -> LegalisationRequirements {
    let mut requirements = LegalisationRequirements::new();
    requirements.values.insert(ValueFeature::FixedWidthIntegers);
    requirements.memory.insert(MemoryFeature::MutableLocals);
    requirements
}

fn packet_requirements() -> LegalisationRequirements {
    let mut requirements = scalar_requirements();
    requirements.memory.insert(MemoryFeature::VerifierPointers);
    requirements
}

fn map_requirements() -> LegalisationRequirements {
    packet_requirements()
}

fn verdict_requirements() -> LegalisationRequirements {
    let mut requirements = LegalisationRequirements::new();
    requirements.values.insert(ValueFeature::FixedWidthIntegers);
    requirements
}

fn semantic_failures(facts: &InvocationFacts) -> Vec<EbpfSemanticFailure> {
    let mut failures = Vec::new();
    if !exact_ok_completion(facts) {
        failures.push(EbpfSemanticFailure::CompletionNotExactOk);
    }
    if !matches!(&facts.state_transitions, StateTransitionKnowledge::Declared(transitions) if transitions.facts().is_empty())
    {
        failures.push(EbpfSemanticFailure::StateTransitionsNotEmptyAndClosed);
    }
    if facts.effects.callback() != tcl_registry::CallbackEffect::NONE {
        failures.push(EbpfSemanticFailure::CallbackOrReentrancy);
    }
    let legacy = facts.effects.legacy();
    if !facts.effects.accesses().is_empty()
        || !legacy.command_table_effects.is_empty()
        || !legacy.frame_effects.is_empty()
        || !legacy.side_effects.is_empty()
    {
        failures.push(EbpfSemanticFailure::TclWorldAccessOrLegacyEffect);
    }
    failures
}

fn exact_ok_completion(facts: &InvocationFacts) -> bool {
    matches!(facts.completion.codes, CompletionCodeDomain::Exact(codes) if codes == [CompletionCode::Ok])
        && facts.completion.payloads == CompletionPayloadObligations::PRODUCED
}

/// A sealed descriptor-level eBPF region.  It has no emitter or lowering API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SealedEbpfRegion {
    /// Every invocation audited as direct eBPF input, in source execution order.
    pub invocations: Vec<SealedEbpfInvocation>,
}

/// One invocation that passed the bridge's conservative semantic audit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SealedEbpfInvocation {
    /// Deterministic invocation position in the common executable function.
    pub index: usize,
    /// Canonical registry command identity for diagnostics and audit output.
    pub canonical_command: String,
    /// Descriptor-derived target key; never `SemanticOperationId::Invoke`.
    pub operation: EbpfOperationKey,
}

/// The result of auditing one candidate BPF-Tcl region.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EbpfRegionEligibility {
    /// Every invocation is closed and eligible for future direct eBPF lowering.
    Sealed(SealedEbpfRegion),
    /// Common analysis or one or more invocations prevented sealing.
    Declined(EbpfRegionDecline),
}

/// Why the bridge could not seal a candidate region.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EbpfRegionDecline {
    /// The semantic bundle carried no resolved environment (ledger C1 /
    /// redesign §11.2 D1: the retired form carried the mask that named no one
    /// profile; an absent context is the whole of that state now).
    ContextUnavailable,
    /// The source was outside the current executable semantic subset.
    SourceDeclined(tcl_compiler::executable_ir::SourceCompatibilityDecline),
    /// Common world-state SSA could not be built for the executable function.
    WorldStateDeclined(tcl_compiler::world_state_ssa::WorldStateSsaDecline),
    /// The interactive bundle retained invocation facts but did not request
    /// the world-state proof graph required by this whole-region auditor.
    WorldStateNotRequested,
    /// The enclosing function build did not retain source semantic IR.
    SourceUnavailable,
    /// The common executable region contains no-runtime-incompatible shape.
    ExecutableShape(Vec<EbpfExecutableShapeDecline>),
    /// One or more executable invocations failed the direct eBPF audit.
    Invocations(Vec<EbpfInvocationDecline>),
}

/// Why the common executable region is not the supported staged argv/invoke shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EbpfExecutableShapeDecline {
    /// A future common instruction is not explicitly part of the staged eBPF bridge.
    NonInvocationInstruction {
        /// Owning executable block index.
        block: usize,
        /// Instruction position within `block`.
        instruction: usize,
    },
    /// A branch, jump, or malformed terminator has no no-runtime eBPF bridge yet.
    UnsupportedControlFlow {
        /// Owning executable block index.
        block: usize,
        /// Unsupported control-flow shape.
        kind: EbpfControlFlowKind,
    },
    /// The source region was non-empty, but no generic invocation was audited.
    NoAuditedInvocations,
}

/// A common executable control-flow shape not yet admitted by the eBPF bridge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EbpfControlFlowKind {
    /// An unconditional CFG edge.
    Goto,
    /// A value-dependent CFG branch.
    Branch,
    /// An incomplete executable block.
    MissingTerminator,
}

/// One invocation's typed reason for preventing region sealing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EbpfInvocationDecline {
    /// Deterministic invocation position in the common executable function.
    pub index: usize,
    /// Canonical registry identity when registry resolution completed.
    pub canonical_command: Option<String>,
    /// The reason this invocation cannot be direct eBPF input.
    pub reason: EbpfInvocationDeclineReason,
}

/// One typed reason an invocation cannot enter a sealed eBPF region.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EbpfInvocationDeclineReason {
    /// The command head was dynamic or absent from the selected registry dialect.
    Unresolved(OwnedInvocationResolutionUnresolved),
    /// The resolved registry command has no BPF-Tcl operation descriptor.
    NotBpfOperation,
    /// The BPF descriptor is a frontend declaration or macro, not an executable op.
    DescriptorKind(EbpfOperationKindDecline),
    /// Common semantic facts were not closed enough for no-runtime eBPF execution.
    Semantic(Vec<EbpfSemanticFailure>),
}

/// A target-independent semantic condition that direct eBPF lowering requires.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EbpfSemanticFailure {
    /// The registry did not prove a produced, exact `TCL_OK` completion.
    CompletionNotExactOk,
    /// The invocation has no closed empty Tcl-world transition declaration.
    StateTransitionsNotEmptyAndClosed,
    /// The invocation may call Tcl or host code, or re-enter an interpreter.
    CallbackOrReentrancy,
    /// The invocation reads or mutates Tcl-world state, or retains a legacy
    /// effect bridge. Packet and map access must instead be in `BpfOpSpec`.
    TclWorldAccessOrLegacyEffect,
    /// The eBPF target contract has no direct legalisation path.
    TargetLegalisation(LegalisationFailure),
    /// A future contract registration unexpectedly selected a non-direct path.
    NonDirectTargetLegalisation(LegalisationStrategy),
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::lowering::lower_to_ir;
    use tcl_registry::model::ingress::static_context_for;
    use tcl_registry::model::semantic::SemanticContext;

    fn bpf_registry() -> &'static CommandRegistry {
        static_context_for("bpf").commands()
    }

    fn bpf_context() -> SemanticContext {
        SemanticContext::for_environment("bpf")
    }

    #[test]
    fn bpf_descriptor_alone_never_seals_a_generic_tcl_invocation() {
        let registry = bpf_registry();
        let module = lower_to_ir("setint result {1}", registry);
        let bundle = SemanticAnalysisBundle::build(
            registry,
            Some(bpf_context()),
            &module.top_level,
            tcl_compiler::dispatch_proof::DispatchEntryAssumption::PristineRegistryWorld,
        );

        let EbpfRegionEligibility::Declined(EbpfRegionDecline::Invocations(declines)) =
            EbpfSemanticBridge::new().assess(registry, &bundle)
        else {
            panic!("a BPF descriptor alone must not certify direct eBPF");
        };
        assert_eq!(declines.len(), 1);
        assert_eq!(declines[0].canonical_command.as_deref(), Some("setint"));
        assert!(matches!(
            &declines[0].reason,
            EbpfInvocationDeclineReason::Semantic(_)
        ));
    }

    #[test]
    fn non_bpf_lowered_statement_retains_a_typed_shape_decline() {
        let registry = bpf_registry();
        let module = lower_to_ir("set result 1", registry);
        let bundle = SemanticAnalysisBundle::build(
            registry,
            Some(bpf_context()),
            &module.top_level,
            tcl_compiler::dispatch_proof::DispatchEntryAssumption::PristineRegistryWorld,
        );

        let EbpfRegionEligibility::Declined(EbpfRegionDecline::ExecutableShape(declines)) =
            EbpfSemanticBridge::new().assess(registry, &bundle)
        else {
            panic!("ordinary lowered Tcl must retain its typed executable-shape decline");
        };
        assert!(matches!(
            declines.as_slice(),
            [
                EbpfExecutableShapeDecline::NonInvocationInstruction { .. },
                EbpfExecutableShapeDecline::NoAuditedInvocations,
            ]
        ));
    }

    #[test]
    fn available_zero_invocation_function_cannot_be_sealed() {
        use tcl_compiler::executable_ir::{
            CompletionId, ExecutableBlock, ExecutableBlockId, ExecutableFunction,
            ExecutableFunctionId, ExecutableInstruction, ExecutableTerminator, ExecutableValueId,
        };
        use tcl_compiler::ir::{NodeId, SourceSite, WordExpr};

        let function_id = ExecutableFunctionId::new(7);
        let block_id = ExecutableBlockId::new(function_id, 0);
        let completion = CompletionId::new(function_id, 0);
        let value = ExecutableValueId::new(function_id, 0);
        let source = SourceSite::source(tcl_lexer::Span::empty(0));
        let function = ExecutableFunction::new(
            function_id,
            block_id,
            vec![ExecutableBlock {
                id: block_id,
                instructions: vec![ExecutableInstruction::EvaluateWord {
                    value,
                    completion,
                    word: WordExpr::Literal {
                        text: "literal".to_owned(),
                        source: source.clone(),
                    },
                    node: NodeId::from_path(vec![0]),
                    source,
                }],
                terminator: Some(ExecutableTerminator::ReturnCompletion(completion)),
            }],
        );
        assert!(function.validate().is_ok());

        assert_eq!(
            audit_executable_shape(&function),
            vec![EbpfExecutableShapeDecline::NoAuditedInvocations]
        );
    }

    #[test]
    fn target_contract_is_keyed_by_descriptor_semantics_not_invoke() {
        let bridge = EbpfSemanticBridge::new();
        assert!(matches!(
            bridge.target().assess(&EbpfOperationKey::Scalar),
            LegalisationDecision::Selected(LegalisationStrategy::Direct)
        ));
    }

    #[test]
    fn any_common_tcl_world_access_prevents_sealing() {
        let registry = bpf_registry();
        let module = lower_to_ir("setint result {1}", registry);
        let bundle = SemanticAnalysisBundle::build(
            registry,
            Some(bpf_context()),
            &module.top_level,
            tcl_compiler::dispatch_proof::DispatchEntryAssumption::PristineRegistryWorld,
        );
        let mut facts = first_resolved_facts(&bundle);
        facts
            .effects
            .add_access(tcl_registry::EffectAccess::variable_store_wildcard(
                tcl_registry::EffectAccessMode::Read,
            ));

        assert!(
            semantic_failures(&facts).contains(&EbpfSemanticFailure::TclWorldAccessOrLegacyEffect)
        );
    }

    fn first_resolved_facts(bundle: &SemanticAnalysisBundle) -> InvocationFacts {
        let Some(function) = bundle.executable().function() else {
            panic!("test source should construct executable facts");
        };
        function
            .blocks
            .iter()
            .flat_map(|block| block.instructions.iter())
            .find_map(|instruction| match instruction {
                tcl_compiler::executable_ir::ExecutableInstruction::Invoke(invoke) => {
                    match &invoke.resolution {
                        InvocationResolution::Resolved(facts) => Some((**facts).clone()),
                        InvocationResolution::Unresolved(_) => None,
                    }
                }
                tcl_compiler::executable_ir::ExecutableInstruction::EvaluateWord { .. }
                | tcl_compiler::executable_ir::ExecutableInstruction::ExpandWord { .. }
                | tcl_compiler::executable_ir::ExecutableInstruction::BuildArgv { .. }
                | tcl_compiler::executable_ir::ExecutableInstruction::ExecuteLowered(_)
                | tcl_compiler::executable_ir::ExecutableInstruction::ExecuteOpaqueRegion(_) => {
                    None
                }
            })
            .expect("test source should resolve its BPF operation")
    }
}
