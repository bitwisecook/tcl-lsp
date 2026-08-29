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

//! Target-family capabilities and operation legalisation contracts.
//!
//! This module is the target-neutral seam between the common semantic
//! compiler and target-family lowerers.  It deliberately contains **no Tcl
//! command names, registry lookups, or emitter callbacks**.  The
//! [`tcl_registry::CommandRegistry`](tcl_registry::CommandRegistry) remains
//! the source of truth for command semantics; a future common semantic IR
//! supplies a typed [`SemanticOperationKey`] for each resolved operation.
//! Backends then register how their target can legalise that operation.
//!
//! This is distinct from `tcl_platform::Capabilities`: platform capabilities
//! describe facilities granted to one running interpreter, such as filesystem
//! or socket access. This module describes whether a target family can
//! represent a semantic operation at all. A legal target path still delegates
//! host-authorisation checks to the shared runtime.
//!
//! A target is described by six orthogonal facets rather than a flat structure
//! of `supports_*` booleans:
//!
//! - [`ExecutionCapabilities`] describes its execution model and scheduling;
//! - [`ValueCapabilities`] describes values it can represent;
//! - [`ControlCapabilities`] describes its control-flow model;
//! - [`MemoryCapabilities`] describes its memory model;
//! - [`RuntimeCapabilities`] describes runtime services it can call; and
//! - [`ResourceLimits`] describes quantitative resource bounds.
//!
//! The contract is intentionally useful before the executable semantic IR is
//! introduced: callers may use a small local operation enum in tests or an
//! experimental backend.  Once `SemanticOpId` exists, that type implements
//! [`SemanticOperationKey`] and becomes the only production map key.

use std::collections::{BTreeMap, BTreeSet};
use tcl_dialect::model::Family;

/// Marker for a target-neutral operation identity from the common semantic IR.
///
/// Implement this for an operation ID, not for Tcl command spelling.  A
/// registry-resolved command form may select one operation; aliases, renames,
/// and dynamic invocation are intentionally not target-contract keys.
pub trait SemanticOperationKey: Ord {}

/// The registry-owned target-neutral operation identity is the production key
/// for compiler target contracts. It deliberately identifies semantic
/// operation forms, not runtime command objects or command spellings.
impl SemanticOperationKey for tcl_registry::SemanticOperationId {}

/// The broad family of a compilation target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TargetFamily {
    /// Tcl bytecode consumed by the native `TclVM` runtime.
    TclVm,
    /// A WebAssembly module coupled with the Tcl runtime ABI.
    Wasm,
    /// A conventional native CPU target such as x86-64, `AArch64`, or RISC-V.
    Native,
    /// A verifier-constrained eBPF kernel program.
    Ebpf,
    /// A data-parallel GPU kernel plus its host launch boundary.
    Gpu,
    /// A spatial FPGA/dataflow circuit plus its host control boundary.
    Fpga,
}

/// The fundamental execution model of a target family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ExecutionModel {
    /// A stateful interpreter or managed module that may execute Tcl runtime
    /// services between operations.
    StatefulInterpreter,
    /// A conventional native thread with ordinary calls and a native stack.
    NativeThread,
    /// A verifier-constrained kernel program, such as eBPF.
    VerifiedKernel,
    /// A single-program, multiple-thread kernel, such as a GPU compute kernel.
    SimtKernel,
    /// A statically scheduled spatial/dataflow pipeline.
    SpatialPipeline,
}

/// A scheduling or invocation facility supplied by an [`ExecutionModel`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ExecutionFeature {
    /// The target can perform a normal direct call.
    DirectCalls,
    /// The target can perform re-entrant calls into a mutable runtime.
    ReentrantRuntimeCalls,
    /// The target can dispatch dynamically after evaluating a command head.
    DynamicDispatch,
    /// The target can execute independently scheduled parallel work items.
    ParallelWorkItems,
    /// The target can synchronise a group of parallel work items.
    WorkgroupSynchronisation,
    /// The target executes a statically scheduled pipeline rather than an
    /// instruction stream.
    StaticScheduling,
}

/// The primary value representation model of a target family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ValueModel {
    /// Tcl objects with faithful dual string/internal representations.
    TclObjects,
    /// Tagged or boxed values with a Tcl object boundary where required.
    TaggedValues,
    /// Native scalar values with an optional boxed Tcl boundary.
    NativeScalars,
    /// Verifier-tracked scalar and pointer values.
    VerifierValues,
    /// Device scalar/vector values in GPU address spaces.
    DeviceValues,
    /// Fixed-width bit vectors and statically shaped aggregates.
    BitVectors,
}

/// A value-representation facility provided by a target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ValueFeature {
    /// The target can represent a Tcl object with identity and dual reps.
    TclObjectAbi,
    /// The target can represent a tagged dynamic value.
    TaggedValues,
    /// The target can represent variable-length strings or byte arrays.
    VariableLengthValues,
    /// The target can represent object/reference identity.
    ReferenceIdentity,
    /// The target can represent arbitrary-precision integer values.
    ArbitraryPrecisionIntegers,
    /// The target can represent fixed-width integer values.
    FixedWidthIntegers,
    /// The target can represent IEEE floating-point values.
    FloatingPoint,
    /// The target can represent vector values.
    VectorValues,
}

/// The fundamental control-flow model of a target family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ControlModel {
    /// Arbitrary control-flow graphs and runtime calls.
    ArbitraryCfg,
    /// Structured control flow with a finite instruction representation.
    StructuredCfg,
    /// A verifier requires statically bounded loops and control flow.
    BoundedCfg,
    /// Kernel control flow with SIMT divergence rules.
    KernelCfg,
    /// A static graph of dataflow and pipeline stages.
    StaticDataflow,
}

/// A control-flow facility supplied by a target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ControlFeature {
    /// The target accepts arbitrary control-flow joins and backedges.
    ArbitraryCfg,
    /// The target accepts structured branches and loops.
    StructuredControl,
    /// Every loop must have a statically proven bound.
    BoundedLoops,
    /// The target supports recursive calls.
    Recursion,
    /// The target can propagate Tcl's non-local completion values.
    NonLocalCompletion,
    /// The target can suspend and resume execution.
    Suspension,
    /// The target can return a single kernel or pipeline status to its host.
    KernelStatus,
}

/// The fundamental memory model of a target family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MemoryModel {
    /// Interpreter-managed heap, frames, and namespace storage.
    InterpreterHeap,
    /// Linear memory addressed through offsets, as in WebAssembly.
    LinearMemory,
    /// Ordinary process address space and native pointers.
    NativeAddressSpace,
    /// Verifier-tracked stack, map values, packet data, and context pointers.
    VerifierMemory,
    /// Global, shared, local, and constant GPU address spaces.
    DeviceAddressSpaces,
    /// Explicit memories, streams, and channels in a spatial design.
    SpatialMemories,
}

/// A memory facility supplied by a target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MemoryFeature {
    /// The target has a managed heap.
    ManagedHeap,
    /// The target can address linear memory through offsets.
    LinearAddressing,
    /// The target can use native pointers.
    NativePointers,
    /// The target can use verifier-tracked pointer classes.
    VerifierPointers,
    /// The target exposes multiple device memory address spaces.
    AddressSpaces,
    /// The target can maintain observable Tcl frames.
    ObservableFrames,
    /// The target can represent dynamic variable aliases such as `upvar`.
    DynamicAliases,
    /// The target can allocate mutable local storage.
    MutableLocals,
}

/// The runtime relationship supplied by a target family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RuntimeModel {
    /// A complete Tcl runtime is available in the target execution domain.
    FullTclRuntime,
    /// A deliberately restricted Tcl-compatible runtime is available.
    RestrictedTclRuntime,
    /// Unsupported work may transfer to a host continuation.
    HostContinuation,
    /// No Tcl runtime exists in the target execution domain.
    NoRuntime,
}

/// A runtime service that a semantic operation may require.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RuntimeService {
    /// Generic argv invocation after dynamic command resolution.
    GenericInvoke,
    /// Script evaluation through Tcl's `eval` machinery.
    Eval,
    /// Dynamic command and namespace resolution.
    DynamicCommandResolution,
    /// The Tcl variable, execution, and command trace dispatcher.
    TraceDispatcher,
    /// Observable frame, variable, and namespace introspection.
    FrameIntrospection,
    /// Tcl completion code, result, and options propagation.
    TclCompletion,
    /// Coroutine or other suspension/resumption support.
    Suspension,
    /// A target-specific runtime intrinsic selected below common semantics.
    Intrinsics,
    /// Transfer an enclosing offloaded region to a host continuation.
    HostContinuation,
}

/// A deterministic set of finite capability values.
///
/// This avoids a growing set of unrelated `supports_*` flags while retaining
/// exhaustive enums for each capability domain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeatureSet<T> {
    values: BTreeSet<T>,
}

impl<T: Ord> FeatureSet<T> {
    /// Create an empty feature set.
    #[must_use]
    pub fn new() -> Self {
        Self {
            values: BTreeSet::new(),
        }
    }

    /// Construct a set from finite capability values.
    #[must_use]
    pub fn from_features(features: impl IntoIterator<Item = T>) -> Self {
        Self {
            values: features.into_iter().collect(),
        }
    }

    /// Add a feature to this set.
    pub fn insert(&mut self, feature: T) -> bool {
        self.values.insert(feature)
    }

    /// Whether this set contains `feature`.
    #[must_use]
    pub fn contains(&self, feature: &T) -> bool {
        self.values.contains(feature)
    }

    /// Whether every feature in `required` occurs in this set.
    #[must_use]
    pub fn contains_all(&self, required: &Self) -> bool {
        required.values.is_subset(&self.values)
    }

    /// Iterate over features in deterministic order.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.values.iter()
    }

    /// Whether the set contains no features.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

impl<T: Ord> Default for FeatureSet<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Execution capabilities of a target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionCapabilities {
    model: ExecutionModel,
    features: FeatureSet<ExecutionFeature>,
}

impl ExecutionCapabilities {
    /// Construct an execution capability description.
    #[must_use]
    pub fn new(
        model: ExecutionModel,
        features: impl IntoIterator<Item = ExecutionFeature>,
    ) -> Self {
        Self {
            model,
            features: FeatureSet::from_features(features),
        }
    }

    /// The target's fundamental execution model.
    #[must_use]
    pub const fn model(&self) -> ExecutionModel {
        self.model
    }

    /// The scheduling and invocation facilities this target provides.
    #[must_use]
    pub const fn features(&self) -> &FeatureSet<ExecutionFeature> {
        &self.features
    }
}

/// Value capabilities of a target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueCapabilities {
    model: ValueModel,
    features: FeatureSet<ValueFeature>,
}

impl ValueCapabilities {
    /// Construct a value capability description.
    #[must_use]
    pub fn new(model: ValueModel, features: impl IntoIterator<Item = ValueFeature>) -> Self {
        Self {
            model,
            features: FeatureSet::from_features(features),
        }
    }

    /// The target's fundamental value model.
    #[must_use]
    pub const fn model(&self) -> ValueModel {
        self.model
    }

    /// The value features this target provides.
    #[must_use]
    pub const fn features(&self) -> &FeatureSet<ValueFeature> {
        &self.features
    }
}

/// Control-flow capabilities of a target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlCapabilities {
    model: ControlModel,
    features: FeatureSet<ControlFeature>,
}

impl ControlCapabilities {
    /// Construct a control-flow capability description.
    #[must_use]
    pub fn new(model: ControlModel, features: impl IntoIterator<Item = ControlFeature>) -> Self {
        Self {
            model,
            features: FeatureSet::from_features(features),
        }
    }

    /// The target's fundamental control-flow model.
    #[must_use]
    pub const fn model(&self) -> ControlModel {
        self.model
    }

    /// The control-flow features this target provides.
    #[must_use]
    pub const fn features(&self) -> &FeatureSet<ControlFeature> {
        &self.features
    }
}

/// Memory capabilities of a target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryCapabilities {
    model: MemoryModel,
    features: FeatureSet<MemoryFeature>,
}

impl MemoryCapabilities {
    /// Construct a memory capability description.
    #[must_use]
    pub fn new(model: MemoryModel, features: impl IntoIterator<Item = MemoryFeature>) -> Self {
        Self {
            model,
            features: FeatureSet::from_features(features),
        }
    }

    /// The target's fundamental memory model.
    #[must_use]
    pub const fn model(&self) -> MemoryModel {
        self.model
    }

    /// The memory features this target provides.
    #[must_use]
    pub const fn features(&self) -> &FeatureSet<MemoryFeature> {
        &self.features
    }
}

/// Runtime capabilities of a target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeCapabilities {
    model: RuntimeModel,
    services: FeatureSet<RuntimeService>,
}

impl RuntimeCapabilities {
    /// Construct a runtime capability description.
    #[must_use]
    pub fn new(model: RuntimeModel, services: impl IntoIterator<Item = RuntimeService>) -> Self {
        Self {
            model,
            services: FeatureSet::from_features(services),
        }
    }

    /// The target's runtime relationship.
    #[must_use]
    pub const fn model(&self) -> RuntimeModel {
        self.model
    }

    /// The runtime services this target provides.
    #[must_use]
    pub const fn services(&self) -> &FeatureSet<RuntimeService> {
        &self.services
    }
}

/// A resource class with a target-specific quantitative limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ResourceKind {
    /// Number of instructions or equivalent scheduled operations.
    InstructionCount,
    /// Bytes of call-stack or verifier stack storage.
    StackBytes,
    /// Nesting depth of calls.
    CallDepth,
    /// Largest statically proven loop iteration count.
    LoopIterations,
    /// Concurrent work items in one GPU workgroup or equivalent unit.
    ConcurrentLanes,
    /// Bytes of local, shared, or scratchpad memory.
    LocalMemoryBytes,
    /// Number of FPGA pipeline stages or equivalent scheduled stages.
    PipelineStages,
}

/// A target's bound for one [`ResourceKind`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceBound {
    /// The target has no meaningful fixed bound in this contract.
    Unbounded,
    /// The target accepts at most the given amount.
    AtMost(u64),
    /// The target cannot provide this resource at all.
    Unsupported,
    /// The target may impose a bound, but it is not known at compile time.
    Unknown,
}

/// A minimum amount of one resource required to legalise an operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceDemand {
    kind: ResourceKind,
    amount: u64,
}

impl ResourceDemand {
    /// Construct a resource demand.
    #[must_use]
    pub const fn new(kind: ResourceKind, amount: u64) -> Self {
        Self { kind, amount }
    }

    /// The demanded resource class.
    #[must_use]
    pub const fn kind(&self) -> ResourceKind {
        self.kind
    }

    /// The minimum amount required.
    #[must_use]
    pub const fn amount(&self) -> u64 {
        self.amount
    }
}

/// Why a target's resource declaration cannot satisfy a demand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceFailure {
    /// The resource is not available on the target.
    Unsupported(ResourceKind),
    /// The target's effective bound is not known at compile time.
    Unknown(ResourceKind),
    /// The requested amount exceeds the target's declared maximum.
    ExceedsLimit {
        /// The resource class being checked.
        kind: ResourceKind,
        /// The target's declared maximum.
        maximum: u64,
        /// The operation's required amount.
        required: u64,
    },
}

/// Quantitative resource limits supplied by a target.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ResourceLimits {
    bounds: BTreeMap<ResourceKind, ResourceBound>,
}

impl ResourceLimits {
    /// Create a resource declaration with no known bounds.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add or replace one resource bound.
    #[must_use]
    pub fn with_bound(mut self, kind: ResourceKind, bound: ResourceBound) -> Self {
        self.bounds.insert(kind, bound);
        self
    }

    /// Return the declared bound, or [`ResourceBound::Unknown`] when omitted.
    #[must_use]
    pub fn bound(&self, kind: ResourceKind) -> ResourceBound {
        self.bounds
            .get(&kind)
            .copied()
            .unwrap_or(ResourceBound::Unknown)
    }

    /// Check whether this target can satisfy `demand`.
    pub fn check(&self, demand: ResourceDemand) -> Result<(), ResourceFailure> {
        match self.bound(demand.kind) {
            ResourceBound::Unbounded => Ok(()),
            ResourceBound::AtMost(maximum) if demand.amount <= maximum => Ok(()),
            ResourceBound::AtMost(maximum) => Err(ResourceFailure::ExceedsLimit {
                kind: demand.kind,
                maximum,
                required: demand.amount,
            }),
            ResourceBound::Unsupported => Err(ResourceFailure::Unsupported(demand.kind)),
            ResourceBound::Unknown => Err(ResourceFailure::Unknown(demand.kind)),
        }
    }
}

/// The complete non-command capability description of one target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetCapabilities {
    execution: ExecutionCapabilities,
    values: ValueCapabilities,
    control: ControlCapabilities,
    memory: MemoryCapabilities,
    runtime: RuntimeCapabilities,
    resources: ResourceLimits,
}

impl TargetCapabilities {
    /// Construct a complete target capability description.
    #[must_use]
    pub fn new(
        execution: ExecutionCapabilities,
        values: ValueCapabilities,
        control: ControlCapabilities,
        memory: MemoryCapabilities,
        runtime: RuntimeCapabilities,
        resources: ResourceLimits,
    ) -> Self {
        Self {
            execution,
            values,
            control,
            memory,
            runtime,
            resources,
        }
    }

    /// Capabilities for the `TclVM` bytecode target and its full runtime.
    #[must_use]
    pub fn tclvm() -> Self {
        Self::full_tcl_runtime(MemoryModel::InterpreterHeap)
    }

    /// Capabilities for the WASM target linked with the Rust Tcl runtime.
    #[must_use]
    pub fn wasm() -> Self {
        Self::full_tcl_runtime(MemoryModel::LinearMemory)
    }

    /// Capabilities for a conventional native CPU target with a full Tcl runtime.
    #[must_use]
    pub fn native() -> Self {
        Self::new(
            ExecutionCapabilities::new(
                ExecutionModel::NativeThread,
                [
                    ExecutionFeature::DirectCalls,
                    ExecutionFeature::ReentrantRuntimeCalls,
                    ExecutionFeature::DynamicDispatch,
                ],
            ),
            ValueCapabilities::new(
                ValueModel::NativeScalars,
                [
                    ValueFeature::TclObjectAbi,
                    ValueFeature::TaggedValues,
                    ValueFeature::VariableLengthValues,
                    ValueFeature::ReferenceIdentity,
                    ValueFeature::ArbitraryPrecisionIntegers,
                    ValueFeature::FixedWidthIntegers,
                    ValueFeature::FloatingPoint,
                ],
            ),
            Self::full_control(),
            MemoryCapabilities::new(
                MemoryModel::NativeAddressSpace,
                [
                    MemoryFeature::ManagedHeap,
                    MemoryFeature::NativePointers,
                    MemoryFeature::ObservableFrames,
                    MemoryFeature::DynamicAliases,
                    MemoryFeature::MutableLocals,
                ],
            ),
            Self::full_runtime(),
            Self::unbounded_resources(),
        )
    }

    /// Capabilities for verifier-constrained eBPF programs.
    #[must_use]
    pub fn ebpf() -> Self {
        Self::new(
            ExecutionCapabilities::new(
                ExecutionModel::VerifiedKernel,
                [ExecutionFeature::DirectCalls],
            ),
            ValueCapabilities::new(
                ValueModel::VerifierValues,
                [ValueFeature::FixedWidthIntegers],
            ),
            ControlCapabilities::new(
                ControlModel::BoundedCfg,
                [
                    ControlFeature::StructuredControl,
                    ControlFeature::BoundedLoops,
                ],
            ),
            MemoryCapabilities::new(
                MemoryModel::VerifierMemory,
                [
                    MemoryFeature::VerifierPointers,
                    MemoryFeature::MutableLocals,
                ],
            ),
            RuntimeCapabilities::new(RuntimeModel::NoRuntime, []),
            ResourceLimits::new()
                .with_bound(ResourceKind::StackBytes, ResourceBound::AtMost(512))
                .with_bound(ResourceKind::CallDepth, ResourceBound::AtMost(8))
                .with_bound(ResourceKind::LoopIterations, ResourceBound::Unknown),
        )
    }

    /// Capabilities for GPU kernels whose enclosing candidate region may remain
    /// on the host when it cannot be lowered.
    #[must_use]
    pub fn gpu() -> Self {
        Self::new(
            ExecutionCapabilities::new(
                ExecutionModel::SimtKernel,
                [
                    ExecutionFeature::DirectCalls,
                    ExecutionFeature::ParallelWorkItems,
                    ExecutionFeature::WorkgroupSynchronisation,
                ],
            ),
            ValueCapabilities::new(
                ValueModel::DeviceValues,
                [
                    ValueFeature::FixedWidthIntegers,
                    ValueFeature::FloatingPoint,
                    ValueFeature::VectorValues,
                ],
            ),
            ControlCapabilities::new(
                ControlModel::KernelCfg,
                [
                    ControlFeature::StructuredControl,
                    ControlFeature::KernelStatus,
                ],
            ),
            MemoryCapabilities::new(
                MemoryModel::DeviceAddressSpaces,
                [MemoryFeature::AddressSpaces, MemoryFeature::MutableLocals],
            ),
            RuntimeCapabilities::new(
                RuntimeModel::HostContinuation,
                [RuntimeService::HostContinuation],
            ),
            ResourceLimits::new()
                .with_bound(ResourceKind::ConcurrentLanes, ResourceBound::Unknown)
                .with_bound(ResourceKind::LocalMemoryBytes, ResourceBound::Unknown),
        )
    }

    /// Capabilities for FPGA/dataflow targets whose enclosing candidate region
    /// may remain on the host when it cannot be lowered.
    #[must_use]
    pub fn fpga() -> Self {
        Self::new(
            ExecutionCapabilities::new(
                ExecutionModel::SpatialPipeline,
                [ExecutionFeature::StaticScheduling],
            ),
            ValueCapabilities::new(ValueModel::BitVectors, [ValueFeature::FixedWidthIntegers]),
            ControlCapabilities::new(ControlModel::StaticDataflow, [ControlFeature::KernelStatus]),
            MemoryCapabilities::new(MemoryModel::SpatialMemories, [MemoryFeature::MutableLocals]),
            RuntimeCapabilities::new(
                RuntimeModel::HostContinuation,
                [RuntimeService::HostContinuation],
            ),
            ResourceLimits::new()
                .with_bound(ResourceKind::PipelineStages, ResourceBound::Unknown)
                .with_bound(ResourceKind::LocalMemoryBytes, ResourceBound::Unknown),
        )
    }

    /// The target's execution capabilities.
    #[must_use]
    pub const fn execution(&self) -> &ExecutionCapabilities {
        &self.execution
    }

    /// The target's value capabilities.
    #[must_use]
    pub const fn values(&self) -> &ValueCapabilities {
        &self.values
    }

    /// The target's control-flow capabilities.
    #[must_use]
    pub const fn control(&self) -> &ControlCapabilities {
        &self.control
    }

    /// The target's memory capabilities.
    #[must_use]
    pub const fn memory(&self) -> &MemoryCapabilities {
        &self.memory
    }

    /// The target's runtime capabilities.
    #[must_use]
    pub const fn runtime(&self) -> &RuntimeCapabilities {
        &self.runtime
    }

    /// The target's quantitative resource limits.
    #[must_use]
    pub const fn resources(&self) -> &ResourceLimits {
        &self.resources
    }

    fn full_tcl_runtime(memory_model: MemoryModel) -> Self {
        let memory_features = match memory_model {
            MemoryModel::InterpreterHeap => [
                MemoryFeature::ManagedHeap,
                MemoryFeature::ObservableFrames,
                MemoryFeature::DynamicAliases,
                MemoryFeature::MutableLocals,
            ],
            MemoryModel::LinearMemory => [
                MemoryFeature::LinearAddressing,
                MemoryFeature::ObservableFrames,
                MemoryFeature::DynamicAliases,
                MemoryFeature::MutableLocals,
            ],
            _ => unreachable!("full Tcl runtime profiles have a known memory model"),
        };
        Self::new(
            ExecutionCapabilities::new(
                ExecutionModel::StatefulInterpreter,
                [
                    ExecutionFeature::DirectCalls,
                    ExecutionFeature::ReentrantRuntimeCalls,
                    ExecutionFeature::DynamicDispatch,
                ],
            ),
            ValueCapabilities::new(
                ValueModel::TclObjects,
                [
                    ValueFeature::TclObjectAbi,
                    ValueFeature::TaggedValues,
                    ValueFeature::VariableLengthValues,
                    ValueFeature::ReferenceIdentity,
                    ValueFeature::ArbitraryPrecisionIntegers,
                    ValueFeature::FixedWidthIntegers,
                    ValueFeature::FloatingPoint,
                ],
            ),
            Self::full_control(),
            MemoryCapabilities::new(memory_model, memory_features),
            Self::full_runtime(),
            Self::unbounded_resources(),
        )
    }

    fn full_control() -> ControlCapabilities {
        ControlCapabilities::new(
            ControlModel::ArbitraryCfg,
            [
                ControlFeature::ArbitraryCfg,
                ControlFeature::StructuredControl,
                ControlFeature::Recursion,
                ControlFeature::NonLocalCompletion,
                ControlFeature::Suspension,
            ],
        )
    }

    fn full_runtime() -> RuntimeCapabilities {
        RuntimeCapabilities::new(
            RuntimeModel::FullTclRuntime,
            [
                RuntimeService::GenericInvoke,
                RuntimeService::Eval,
                RuntimeService::DynamicCommandResolution,
                RuntimeService::TraceDispatcher,
                RuntimeService::FrameIntrospection,
                RuntimeService::TclCompletion,
                RuntimeService::Suspension,
                RuntimeService::Intrinsics,
            ],
        )
    }

    fn unbounded_resources() -> ResourceLimits {
        ResourceLimits::new()
            .with_bound(ResourceKind::InstructionCount, ResourceBound::Unbounded)
            .with_bound(ResourceKind::StackBytes, ResourceBound::Unbounded)
            .with_bound(ResourceKind::CallDepth, ResourceBound::Unbounded)
            .with_bound(ResourceKind::LoopIterations, ResourceBound::Unbounded)
    }
}

/// Requirements that must hold before an operation can use one target path.
///
/// Requirements describe target-independent semantic needs.  They are chosen
/// by common semantic-operation lowering, never by matching a Tcl command name
/// in a backend.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LegalisationRequirements {
    /// Required execution and scheduling features.
    pub execution: FeatureSet<ExecutionFeature>,
    /// Required value-representation features.
    pub values: FeatureSet<ValueFeature>,
    /// Required control-flow features.
    pub control: FeatureSet<ControlFeature>,
    /// Required memory features.
    pub memory: FeatureSet<MemoryFeature>,
    /// Required runtime services.
    pub runtime: FeatureSet<RuntimeService>,
    /// Minimum quantitative resources required by the operation or region.
    pub resources: Vec<ResourceDemand>,
}

impl LegalisationRequirements {
    /// Construct an empty requirement set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Return every unmet requirement in deterministic facet order.
    #[must_use]
    pub fn unmet_by(&self, target: &TargetCapabilities) -> Vec<RequirementFailure> {
        let mut failures = Vec::new();
        failures.extend(
            self.execution
                .iter()
                .filter(|feature| !target.execution().features().contains(feature))
                .copied()
                .map(RequirementFailure::Execution),
        );
        failures.extend(
            self.values
                .iter()
                .filter(|feature| !target.values().features().contains(feature))
                .copied()
                .map(RequirementFailure::Value),
        );
        failures.extend(
            self.control
                .iter()
                .filter(|feature| !target.control().features().contains(feature))
                .copied()
                .map(RequirementFailure::Control),
        );
        failures.extend(
            self.memory
                .iter()
                .filter(|feature| !target.memory().features().contains(feature))
                .copied()
                .map(RequirementFailure::Memory),
        );
        failures.extend(
            self.runtime
                .iter()
                .filter(|service| !target.runtime().services().contains(service))
                .copied()
                .map(RequirementFailure::Runtime),
        );
        failures.extend(
            self.resources
                .iter()
                .copied()
                .filter_map(|demand| target.resources().check(demand).err())
                .map(RequirementFailure::Resource),
        );
        failures
    }

    /// Whether every requirement is satisfied by `target`.
    #[must_use]
    pub fn is_satisfied_by(&self, target: &TargetCapabilities) -> bool {
        self.unmet_by(target).is_empty()
    }
}

/// A specific requirement that a target does not meet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequirementFailure {
    /// A required execution feature is unavailable.
    Execution(ExecutionFeature),
    /// A required value feature is unavailable.
    Value(ValueFeature),
    /// A required control-flow feature is unavailable.
    Control(ControlFeature),
    /// A required memory feature is unavailable.
    Memory(MemoryFeature),
    /// A required runtime service is unavailable.
    Runtime(RuntimeService),
    /// A resource demand is not satisfied.
    Resource(ResourceFailure),
}

/// The target path selected for a legal semantic operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegalisationStrategy {
    /// Lower directly into the target-family IR.
    Direct,
    /// Lower through a target runtime intrinsic.
    RuntimeIntrinsic,
    /// Lower through the runtime's generic argv invocation path.
    GenericInvoke,
    /// Transfer an enclosing offload region and its materialised state to a
    /// host continuation.
    ///
    /// This is a region boundary, not permission to emit an arbitrary
    /// per-operation fallback from GPU or FPGA code.
    HostFallback,
}

impl LegalisationStrategy {
    fn implicit_runtime_service(self) -> Option<RuntimeService> {
        match self {
            Self::Direct => None,
            Self::RuntimeIntrinsic => Some(RuntimeService::Intrinsics),
            Self::GenericInvoke => Some(RuntimeService::GenericInvoke),
            Self::HostFallback => Some(RuntimeService::HostContinuation),
        }
    }
}

/// One ordered target-family legalisation path for a semantic operation.
///
/// A backend normally registers several paths for one operation, such as a
/// native selection followed by a runtime intrinsic and generic invocation.
/// [`TargetContract::assess`] chooses the first path whose requirements hold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationLegalisation {
    strategy: LegalisationStrategy,
    requirements: LegalisationRequirements,
}

impl OperationLegalisation {
    /// Construct a legalisation registration.
    #[must_use]
    pub fn new(strategy: LegalisationStrategy, requirements: LegalisationRequirements) -> Self {
        Self {
            strategy,
            requirements,
        }
    }

    /// The target path selected if all requirements are met.
    #[must_use]
    pub const fn strategy(&self) -> LegalisationStrategy {
        self.strategy
    }

    /// The target-independent requirements for this path.
    #[must_use]
    pub const fn requirements(&self) -> &LegalisationRequirements {
        &self.requirements
    }

    fn unmet_by(&self, target: &TargetCapabilities) -> Vec<RequirementFailure> {
        let mut failures = self.requirements.unmet_by(target);
        if let Some(service) = self.strategy.implicit_runtime_service()
            && !target.runtime().services().contains(&service)
        {
            failures.push(RequirementFailure::Runtime(service));
        }
        failures
    }
}

/// The result of assessing one semantic operation against a target contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LegalisationDecision {
    /// The operation may follow the stated target path.
    Selected(LegalisationStrategy),
    /// The operation is not legal for the target under the registered path.
    Declined(LegalisationFailure),
}

/// Why operation legalisation was declined.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LegalisationFailure {
    /// This backend has no registration for the semantic operation.
    MissingOperation,
    /// Every registered path has unmet target-independent requirements.
    NoLegalisationPath(Vec<StrategyFailure>),
}

/// Per-target legalisation data keyed by common semantic operation identity.
///
/// The map is deliberately generic: the common executable IR will supply the
/// concrete operation ID.  This prevents a target backend from treating Tcl
/// command spelling as its dispatch key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetContract<Op> {
    family: TargetFamily,
    capabilities: TargetCapabilities,
    operations: BTreeMap<Op, Vec<OperationLegalisation>>,
}

impl<Op: SemanticOperationKey> TargetContract<Op> {
    /// Create an empty target contract for one target family.
    #[must_use]
    pub fn new(family: TargetFamily, capabilities: TargetCapabilities) -> Self {
        Self {
            family,
            capabilities,
            operations: BTreeMap::new(),
        }
    }

    /// The target family this contract describes.
    #[must_use]
    pub const fn family(&self) -> TargetFamily {
        self.family
    }

    /// The target's common, non-command capabilities.
    #[must_use]
    pub const fn capabilities(&self) -> &TargetCapabilities {
        &self.capabilities
    }

    /// Add one ordered legalisation path for a semantic operation.
    ///
    /// Paths are examined in registration order. Register the most specialised
    /// direct path first and the generic runtime or host fallback last. A host
    /// fallback path is valid only for an explicit offload-boundary operation;
    /// it never permits an arbitrary fallback inside a device region. The
    /// operation key is semantic, never a Tcl command spelling.
    pub fn register(&mut self, operation: Op, legalisation: OperationLegalisation) {
        self.operations
            .entry(operation)
            .or_default()
            .push(legalisation);
    }

    /// Return the ordered legalisation paths for `operation`, if any.
    #[must_use]
    pub fn operations(&self, operation: &Op) -> Option<&[OperationLegalisation]> {
        self.operations.get(operation).map(Vec::as_slice)
    }

    /// Assess whether this target can legalise `operation`.
    #[must_use]
    pub fn assess(&self, operation: &Op) -> LegalisationDecision {
        let Some(legalisations) = self.operations.get(operation) else {
            return LegalisationDecision::Declined(LegalisationFailure::MissingOperation);
        };
        let mut failures = Vec::new();
        for legalisation in legalisations {
            let unmet = legalisation.unmet_by(&self.capabilities);
            if unmet.is_empty() {
                return LegalisationDecision::Selected(legalisation.strategy());
            }
            failures.push(StrategyFailure {
                strategy: legalisation.strategy(),
                unmet,
            });
        }
        LegalisationDecision::Declined(LegalisationFailure::NoLegalisationPath(failures))
    }
}

/// One legalisation path and the requirements that prevented its selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StrategyFailure {
    /// The path that could not be selected.
    pub strategy: LegalisationStrategy,
    /// Every requirement this path could not satisfy.
    pub unmet: Vec<RequirementFailure>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    enum TestOp {
        /// A pure scalar operation.
        Scalar,
        /// An operation requiring the Tcl trace dispatcher.
        TraceAware,
        /// An operation requiring host fallback.
        OffloadBoundary,
    }

    impl SemanticOperationKey for TestOp {}

    #[test]
    fn full_runtime_targets_support_trace_aware_generic_invocation() {
        let mut contract = TargetContract::new(TargetFamily::Wasm, TargetCapabilities::wasm());
        let mut direct_requirements = LegalisationRequirements::new();
        direct_requirements
            .values
            .insert(ValueFeature::VectorValues);
        contract.register(
            TestOp::TraceAware,
            OperationLegalisation::new(LegalisationStrategy::Direct, direct_requirements),
        );

        let mut requirements = LegalisationRequirements::new();
        requirements.runtime.insert(RuntimeService::TraceDispatcher);
        requirements.runtime.insert(RuntimeService::TclCompletion);
        contract.register(
            TestOp::TraceAware,
            OperationLegalisation::new(LegalisationStrategy::GenericInvoke, requirements),
        );

        assert_eq!(
            contract.assess(&TestOp::TraceAware),
            LegalisationDecision::Selected(LegalisationStrategy::GenericInvoke)
        );
    }

    #[test]
    fn ebpf_declines_runtime_dependent_operations_but_accepts_scalar_ones() {
        let mut contract = TargetContract::new(TargetFamily::Ebpf, TargetCapabilities::ebpf());
        let mut scalar_requirements = LegalisationRequirements::new();
        scalar_requirements
            .values
            .insert(ValueFeature::FixedWidthIntegers);
        scalar_requirements
            .control
            .insert(ControlFeature::BoundedLoops);
        contract.register(
            TestOp::Scalar,
            OperationLegalisation::new(LegalisationStrategy::Direct, scalar_requirements),
        );

        let mut trace_requirements = LegalisationRequirements::new();
        trace_requirements
            .runtime
            .insert(RuntimeService::TraceDispatcher);
        contract.register(
            TestOp::TraceAware,
            OperationLegalisation::new(LegalisationStrategy::RuntimeIntrinsic, trace_requirements),
        );

        assert_eq!(
            contract.assess(&TestOp::Scalar),
            LegalisationDecision::Selected(LegalisationStrategy::Direct)
        );
        assert_eq!(
            contract.assess(&TestOp::TraceAware),
            LegalisationDecision::Declined(LegalisationFailure::NoLegalisationPath(vec![
                StrategyFailure {
                    strategy: LegalisationStrategy::RuntimeIntrinsic,
                    unmet: vec![
                        RequirementFailure::Runtime(RuntimeService::TraceDispatcher),
                        RequirementFailure::Runtime(RuntimeService::Intrinsics),
                    ],
                },
            ]))
        );
    }

    #[test]
    fn host_fallback_is_available_to_offload_targets_only() {
        let requirements = LegalisationRequirements::new();

        let mut gpu = TargetContract::new(TargetFamily::Gpu, TargetCapabilities::gpu());
        gpu.register(
            TestOp::OffloadBoundary,
            OperationLegalisation::new(LegalisationStrategy::HostFallback, requirements.clone()),
        );

        let mut ebpf = TargetContract::new(TargetFamily::Ebpf, TargetCapabilities::ebpf());
        ebpf.register(
            TestOp::OffloadBoundary,
            OperationLegalisation::new(LegalisationStrategy::HostFallback, requirements),
        );

        assert_eq!(
            gpu.assess(&TestOp::OffloadBoundary),
            LegalisationDecision::Selected(LegalisationStrategy::HostFallback)
        );
        assert_eq!(
            ebpf.assess(&TestOp::OffloadBoundary),
            LegalisationDecision::Declined(LegalisationFailure::NoLegalisationPath(vec![
                StrategyFailure {
                    strategy: LegalisationStrategy::HostFallback,
                    unmet: vec![RequirementFailure::Runtime(
                        RuntimeService::HostContinuation,
                    )],
                },
            ]))
        );
    }

    #[test]
    fn resource_limits_are_checked_without_a_boolean_per_limit() {
        let limits =
            ResourceLimits::new().with_bound(ResourceKind::StackBytes, ResourceBound::AtMost(512));

        assert_eq!(
            limits.check(ResourceDemand::new(ResourceKind::StackBytes, 512)),
            Ok(())
        );
        assert_eq!(
            limits.check(ResourceDemand::new(ResourceKind::StackBytes, 513)),
            Err(ResourceFailure::ExceedsLimit {
                kind: ResourceKind::StackBytes,
                maximum: 512,
                required: 513,
            })
        );
        assert_eq!(
            limits.check(ResourceDemand::new(ResourceKind::ConcurrentLanes, 1)),
            Err(ResourceFailure::Unknown(ResourceKind::ConcurrentLanes))
        );
    }

    #[test]
    fn unregistered_operations_do_not_fall_back_implicitly() {
        let contract =
            TargetContract::<TestOp>::new(TargetFamily::TclVm, TargetCapabilities::tclvm());

        assert_eq!(
            contract.assess(&TestOp::Scalar),
            LegalisationDecision::Declined(LegalisationFailure::MissingOperation)
        );
    }

    #[test]
    fn registry_semantic_operation_ids_are_real_contract_keys() {
        let mut contract = TargetContract::<tcl_registry::SemanticOperationId>::new(
            TargetFamily::TclVm,
            TargetCapabilities::tclvm(),
        );
        contract.register(
            tcl_registry::SemanticOperationId::Invoke,
            OperationLegalisation::new(
                LegalisationStrategy::GenericInvoke,
                LegalisationRequirements::new(),
            ),
        );

        assert_eq!(
            contract.assess(&tcl_registry::SemanticOperationId::Invoke),
            LegalisationDecision::Selected(LegalisationStrategy::GenericInvoke)
        );
    }
}
